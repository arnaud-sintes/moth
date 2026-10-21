use wasm_bindgen::JsCast;
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent};

use crate::{Error, Listener};

/// A received frame. Binary frames are delivered as bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	/// A text frame.
	Text(String),
	/// A binary frame, the bytes of its `ArrayBuffer`.
	Binary(Vec<u8>),
}

/// Why and how a socket closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Close {
	/// The close code the browser reports. A close started by [`WebSocket::close`] or by dropping
	/// the socket asks for 1000.
	pub code: u16,
	/// The reason string the browser reports; may be empty.
	pub reason: String,
	/// Whether the browser reports the closing handshake as completed cleanly.
	pub was_clean: bool,
}

/// The connection state of a socket, mirroring the browser's `readyState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
	/// The connection is being established.
	Connecting,
	/// The connection is open: the only state in which sending is attempted.
	Open,
	/// A close has started and has not completed.
	Closing,
	/// The connection is closed.
	Closed,
}

/// Non-owning access to a socket, handed to every callback and safe to keep in other components'
/// callbacks. It cannot keep the connection alive: once the owning [`WebSocket`] is dropped the
/// socket is closed and every send fails with an `Err`.
#[derive(Clone)]
pub struct SocketHandle {
	socket: web_sys::WebSocket,
}

impl SocketHandle {
	/// The socket's current state; anything but connecting, open or closing is reported as
	/// `Closed`.
	pub fn state(&self) -> State {
		match self.socket.ready_state() {
			web_sys::WebSocket::CONNECTING => State::Connecting,
			web_sys::WebSocket::OPEN => State::Open,
			web_sys::WebSocket::CLOSING => State::Closing,
			_ => State::Closed,
		}
	}

	/// Sending is only attempted on an open socket. The browser silently discards data sent on a
	/// closing or closed socket, which would be a failure nobody notices; here it is an error.
	pub fn send_text(&self, text: &str) -> Result<(), Error> {
		self.ensure_open()?;
		Ok(self.socket.send_with_str(text)?)
	}

	/// Send `data` as one binary frame. Like [`SocketHandle::send_text`], an error unless the socket
	/// is open.
	pub fn send_binary(&self, data: &[u8]) -> Result<(), Error> {
		self.ensure_open()?;
		Ok(self.socket.send_with_u8_array(data)?)
	}

	fn ensure_open(&self) -> Result<(), Error> {
		match self.state() {
			State::Open => Ok(()),
			state => Err(Error::new(format!("cannot send: socket is {state:?}"))),
		}
	}
}

/// Callbacks of a [`WebSocket`]. Each receives a [`SocketHandle`], so a callback can answer
/// without having to capture the socket that owns it.
#[derive(Default)]
pub struct Handlers {
	open: Option<Box<dyn FnMut(&SocketHandle)>>,
	message: Option<Box<dyn FnMut(&SocketHandle, Message)>>,
	error: Option<Box<dyn FnMut(&SocketHandle)>>,
	close: Option<Box<dyn FnMut(&SocketHandle, Close)>>,
}

impl Handlers {
	/// No callbacks.
	pub fn new() -> Self {
		Self::default()
	}

	/// Called when the connection opens. Replaces a callback set earlier.
	pub fn on_open(mut self, callback: impl FnMut(&SocketHandle) + 'static) -> Self {
		self.open = Some(Box::new(callback));
		self
	}

	/// Called for each received frame, text or binary. Replaces a callback set earlier.
	pub fn on_message(mut self, callback: impl FnMut(&SocketHandle, Message) + 'static) -> Self {
		self.message = Some(Box::new(callback));
		self
	}

	/// The browser gives no detail on errors; a `close` event always follows.
	pub fn on_error(mut self, callback: impl FnMut(&SocketHandle) + 'static) -> Self {
		self.error = Some(Box::new(callback));
		self
	}

	/// Called when the connection has closed, with the [`Close`] details. Replaces a callback set
	/// earlier.
	pub fn on_close(mut self, callback: impl FnMut(&SocketHandle, Close) + 'static) -> Self {
		self.close = Some(Box::new(callback));
		self
	}
}

/// Client WebSocket with RAII semantics.
///
/// Dropping it first unregisters every callback, then closes the connection: no callback can run
/// after the drop, so none can reach a destroyed owner. Callbacks run to completion even when
/// they provoke further events (see [`Listener`]), and may drop the socket themselves.
pub struct WebSocket {
	handle: SocketHandle,
	listeners: Vec<Listener>,
}

impl WebSocket {
	/// Open a connection to `url` and register `handlers`. Frames are received as bytes, not as
	/// blobs. Fails if the browser refuses the URL or a listener cannot be attached (the socket is
	/// then closed); an unreachable server is reported later through `on_close`, not here.
	pub fn connect(url: &str, handlers: Handlers) -> Result<Self, Error> {
		let socket = web_sys::WebSocket::new(url)?;
		socket.set_binary_type(BinaryType::Arraybuffer);
		let handle = SocketHandle { socket };
		// the socket exists before its listeners: if attaching one fails it must not be left open
		match Self::attach(&handle, handlers) {
			Ok(listeners) => Ok(Self { handle, listeners }),
			Err(error) => {
				let _ = handle.socket.close();
				Err(error)
			}
		}
	}

	fn attach(handle: &SocketHandle, handlers: Handlers) -> Result<Vec<Listener>, Error> {
		let target = handle.socket.clone();
		let mut listeners = Vec::new();

		if let Some(mut callback) = handlers.open {
			let handle = handle.clone();
			listeners.push(Listener::new(&target, "open", move |_| callback(&handle))?);
		}
		if let Some(mut callback) = handlers.message {
			let handle = handle.clone();
			listeners.push(Listener::new(&target, "message", move |event| {
				if let Some(message) = decode(event) {
					callback(&handle, message);
				}
			})?);
		}
		if let Some(mut callback) = handlers.error {
			let handle = handle.clone();
			listeners.push(Listener::new(&target, "error", move |_| callback(&handle))?);
		}
		if let Some(mut callback) = handlers.close {
			let handle = handle.clone();
			listeners.push(Listener::new(&target, "close", move |event| {
				if let Ok(event) = event.dyn_into::<CloseEvent>() {
					callback(
						&handle,
						Close {
							code: event.code(),
							reason: event.reason(),
							was_clean: event.was_clean(),
						},
					);
				}
			})?);
		}
		Ok(listeners)
	}

	/// A non-owning [`SocketHandle`] to this socket.
	pub fn handle(&self) -> SocketHandle {
		self.handle.clone()
	}

	/// The socket's current state, see [`SocketHandle::state`].
	pub fn state(&self) -> State {
		self.handle.state()
	}

	/// Send `text` as one text frame; an error unless the socket is open, see
	/// [`SocketHandle::send_text`].
	pub fn send_text(&self, text: &str) -> Result<(), Error> {
		self.handle.send_text(text)
	}

	/// Send `data` as one binary frame; an error unless the socket is open, see
	/// [`SocketHandle::send_binary`].
	pub fn send_binary(&self, data: &[u8]) -> Result<(), Error> {
		self.handle.send_binary(data)
	}

	/// Start a clean close; `on_close` is still called when it completes.
	pub fn close(&self) -> Result<(), Error> {
		Ok(self.handle.socket.close_with_code(1000)?)
	}
}

impl Drop for WebSocket {
	fn drop(&mut self) {
		self.listeners.clear();
		// closing an already closed socket is fine; nothing to recover from during teardown
		let _ = self.handle.socket.close_with_code(1000);
	}
}

fn decode(event: Event) -> Option<Message> {
	let data = event.dyn_into::<MessageEvent>().ok()?.data();
	if let Some(text) = data.as_string() {
		return Some(Message::Text(text));
	}
	let buffer = data.dyn_into::<js_sys::ArrayBuffer>().ok()?;
	Some(Message::Binary(js_sys::Uint8Array::new(&buffer).to_vec()))
}
