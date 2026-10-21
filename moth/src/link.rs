//! Model replication over a WebSocket (feature `replication`).
//!
//! Wire format: every text frame from the server is the complete model as JSON (a snapshot);
//! every text frame from the client is an action as JSON. The server owns the model, the client
//! only holds a [`Replica`]. A snapshot per change is simple and self-healing (a client that
//! missed one is fixed by the next, a new connection starts with the current model, and so does a
//! reconnection); deltas can replace it if a model gets large enough to matter.

use std::{cell::{Cell, RefCell}, collections::VecDeque, marker::PhantomData, rc::{Rc, Weak}};

use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::JsCast;

use crate::{
	Close, Error, Handlers, Listener, Message, Replica, State, Timeout, WebSocket, WeakReplica,
};

/// What a [`Link`] reports besides applying snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
	/// A connection opened (the first one, or one after a reconnection).
	Open,
	/// The connection ended (not because the link was dropped).
	Closed(Close),
	/// The link will try again: `attempt` counts from 1 after each successful connection, and
	/// `delay_ms` is the wait before it.
	Reconnecting {
		/// Number of this attempt, counting from 1.
		attempt: u32,
		/// Milliseconds to wait before connecting again.
		delay_ms: u32,
	},
	/// A text frame that is not a valid model; the replica was left as it was.
	BadSnapshot(String),
	/// A binary message from the server. The link does not interpret it: a stream (video frames,
	/// say) can ride next to the replicated model on the same socket, and the application decodes it.
	Binary(Vec<u8>),
}

/// How long to wait between reconnection attempts: `initial_ms`, doubling after each failure up to
/// `max_ms`, each wait randomized by ±20% so that clients that lost the same server do not all come
/// back at the same instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
	/// The wait before the first attempt, in milliseconds, before jitter.
	pub initial_ms: u32,
	/// The longest wait, in milliseconds; jitter never takes a wait above it.
	pub max_ms: u32,
}

impl Default for Backoff {
	fn default() -> Self {
		Self { initial_ms: 500, max_ms: 10_000 }
	}
}

impl Backoff {
	/// The wait before attempt number `attempt` (counting from 0), with a random jitter. Never 0,
	/// so a server that closes at once cannot make the page spin.
	pub fn delay_ms(&self, attempt: u32) -> u32 {
		self.delay_with(attempt, js_sys::Math::random())
	}

	/// [`Backoff::delay_ms`] with the random number (in `0..=1`) given, to be checked exactly.
	pub fn delay_with(&self, attempt: u32, random: f64) -> u32 {
		let base = self.initial_ms.saturating_mul(1 << attempt.min(16)).min(self.max_ms);
		let jitter = 0.8 + 0.4 * random.clamp(0.0, 1.0);
		((f64::from(base) * jitter) as u32).min(self.max_ms).max(1)
	}
}

/// Settings for [`Link::connect_with`]. The default does not reconnect.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkOptions {
	/// Reconnect after the connection ends, with this backoff. `None` (the default): never; the
	/// link reports `Closed` and stays closed. With reconnection, a page restored from the
	/// back/forward cache (whose socket does not survive it) reconnects at once.
	pub reconnect: Option<Backoff>,
}

/// Where actions go: whatever socket the link has at the moment.
trait Outbox {
	fn send_text(&self, text: &str) -> Result<(), Error>;
}

struct Shared<M> {
	url: String,
	options: LinkOptions,
	replica: WeakReplica<M>,
	on_event: RefCell<Box<dyn FnMut(LinkEvent)>>,
	pending: RefCell<VecDeque<LinkEvent>>,
	socket: RefCell<Option<WebSocket>>,
	/// Failed attempts since the last connection that opened.
	attempt: Cell<u32>,
	timer: RefCell<Option<Timeout>>,
	page_show: RefCell<Option<Listener>>,
	me: Weak<Shared<M>>,
}

impl<M: DeserializeOwned + Clone + PartialEq + 'static> Shared<M> {
	/// Run-to-completion, like a [`Listener`]: an event raised while the callback runs (it asked for
	/// a reconnection, say) is delivered after it returns, in order.
	fn report(&self, event: LinkEvent) {
		self.pending.borrow_mut().push_back(event);
		let Ok(mut on_event) = self.on_event.try_borrow_mut() else { return };
		loop {
			let next = self.pending.borrow_mut().pop_front();
			let Some(event) = next else { break };
			on_event(event);
		}
	}

	fn open(&self) -> Result<(), Error> {
		let (opened, received, closed) = (self.me.clone(), self.me.clone(), self.me.clone());
		let handlers = Handlers::new()
			.on_open(move |_| {
				if let Some(shared) = opened.upgrade() {
					shared.attempt.set(0);
					shared.report(LinkEvent::Open);
				}
			})
			.on_message(move |_, message| {
				let Some(shared) = received.upgrade() else { return };
				match message {
					Message::Text(text) => match serde_json::from_str::<M>(&text) {
						Ok(model) => {
							shared.replica.apply(model);
						}
						Err(error) => shared.report(LinkEvent::BadSnapshot(error.to_string())),
					},
					Message::Binary(bytes) => shared.report(LinkEvent::Binary(bytes)),
				}
			})
			.on_close(move |_, close| {
				if let Some(shared) = closed.upgrade() {
					shared.report(LinkEvent::Closed(close));
					shared.schedule_reconnection();
				}
			});
		let socket = WebSocket::connect(&self.url, handlers)?;
		// replacing the socket drops the previous one, which may be the one calling us
		let previous = self.socket.borrow_mut().replace(socket);
		drop(previous);
		Ok(())
	}

	fn schedule_reconnection(&self) {
		let Some(backoff) = self.options.reconnect else { return };
		let attempt = self.attempt.get();
		self.attempt.set(attempt + 1);
		let delay_ms = backoff.delay_ms(attempt);
		self.report(LinkEvent::Reconnecting { attempt: attempt + 1, delay_ms });

		let me = self.me.clone();
		let timer = Timeout::after(delay_ms, move || {
			let Some(shared) = me.upgrade() else { return };
			let fired = shared.timer.borrow_mut().take();
			drop(fired);
			if let Err(error) = shared.open() {
				crate::console::error(&format!("cannot reconnect: {error}"));
				shared.schedule_reconnection();
			}
		});
		match timer {
			Ok(timer) => {
				let previous = self.timer.borrow_mut().replace(timer);
				drop(previous);
			}
			Err(error) => crate::console::error(&format!("cannot schedule a reconnection: {error}")),
		}
	}

	fn reconnect(&self) -> Result<(), Error> {
		let timer = self.timer.borrow_mut().take();
		drop(timer);
		self.attempt.set(0);
		// no event for the connection we abandon: the caller asked for this
		let old = self.socket.borrow_mut().take();
		drop(old);
		self.open()
	}
}

impl<M> Outbox for Shared<M> {
	fn send_text(&self, text: &str) -> Result<(), Error> {
		match &*self.socket.borrow() {
			Some(socket) => socket.send_text(text),
			None => Err(Error::new("cannot send: not connected")),
		}
	}
}

/// Connection between a [`Replica`] and the model's owner. Dropping it closes the socket, cancels
/// any pending reconnection, and no snapshot is applied afterwards.
pub struct Link<M, A> {
	shared: Rc<Shared<M>>,
	_actions: PhantomData<fn(&A)>,
}

impl<M, A> Link<M, A>
where
	M: DeserializeOwned + Clone + PartialEq + 'static,
	A: Serialize + 'static,
{
	/// Connect without reconnecting: see [`Link::connect_with`].
	pub fn connect(
		url: &str,
		replica: &Replica<M>,
		on_event: impl FnMut(LinkEvent) + 'static,
	) -> Result<Self, Error> {
		Self::connect_with(url, replica, LinkOptions::default(), on_event)
	}

	/// Fails only if the URL is invalid or the browser refuses to create the socket; a server that
	/// is down is not an error here but a `Closed` event (and, with reconnection, retries).
	pub fn connect_with(
		url: &str,
		replica: &Replica<M>,
		options: LinkOptions,
		on_event: impl FnMut(LinkEvent) + 'static,
	) -> Result<Self, Error> {
		let shared = Rc::new_cyclic(|me| Shared {
			url: url.to_owned(),
			options,
			replica: replica.handle(),
			on_event: RefCell::new(Box::new(on_event)),
			pending: RefCell::new(VecDeque::new()),
			socket: RefCell::new(None),
			attempt: Cell::new(0),
			timer: RefCell::new(None),
			page_show: RefCell::new(None),
			me: me.clone(),
		});
		shared.open()?;

		if options.reconnect.is_some() {
			// a page restored from the back/forward cache has a dead socket and no `close` to say so
			let weak = Rc::downgrade(&shared);
			let window = web_sys::window().ok_or_else(|| Error::new("no window"))?;
			let listener = Listener::new(window.unchecked_ref(), "pageshow", move |event| {
				let restored = event
					.dyn_ref::<web_sys::PageTransitionEvent>()
					.is_some_and(web_sys::PageTransitionEvent::persisted);
				if let (true, Some(shared)) = (restored, weak.upgrade()) {
					if let Err(error) = shared.reconnect() {
						crate::console::error(&format!("cannot reconnect: {error}"));
					}
				}
			})?;
			*shared.page_show.borrow_mut() = Some(listener);
		}
		Ok(Self { shared, _actions: PhantomData })
	}

	/// The state of the current socket (`Closed` between a loss and the next attempt).
	pub fn state(&self) -> State {
		self.shared.socket.borrow().as_ref().map_or(State::Closed, WebSocket::state)
	}

	/// Drop the current connection and open a new one now, without waiting for a backoff (for a
	/// "retry" button, or an `online` event). Works with or without automatic reconnection. The
	/// abandoned connection reports nothing.
	pub fn reconnect(&self) -> Result<(), Error> {
		self.shared.reconnect()
	}

	/// A sender for the controls' callbacks. It always uses the link's current socket, so one taken
	/// before a reconnection keeps working after it.
	pub fn actions(&self) -> Actions<A> {
		let outbox: Weak<dyn Outbox> = Rc::downgrade(&self.shared) as Weak<Shared<M>> as Weak<dyn Outbox>;
		Actions { outbox, _types: PhantomData }
	}
}

/// Sends actions to the model's owner; non-owning, safe to keep in callbacks. Once the [`Link`] is
/// dropped, or while it has no open connection, sending returns an `Err`.
pub struct Actions<A> {
	outbox: Weak<dyn Outbox>,
	_types: PhantomData<fn(&A)>,
}

impl<A> Clone for Actions<A> {
	fn clone(&self) -> Self {
		Self { outbox: self.outbox.clone(), _types: PhantomData }
	}
}

impl<A: Serialize> Actions<A> {
	/// Send `action` to the server as one JSON text frame. Fails if the action cannot be
	/// serialized, if the link was dropped, or if the link has no open socket.
	pub fn send(&self, action: &A) -> Result<(), Error> {
		let json = serde_json::to_string(action).map_err(|error| Error::new(error.to_string()))?;
		match self.outbox.upgrade() {
			Some(outbox) => outbox.send_text(&json),
			None => Err(Error::new("cannot send: the link was dropped")),
		}
	}
}
