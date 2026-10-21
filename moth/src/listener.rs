use std::{cell::{Cell, RefCell}, collections::VecDeque, rc::Rc};

use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, EventTarget};

use crate::Error;

/// RAII event listener: registered on creation, unregistered on drop.
///
/// Owns its closure, so a callback can never outlive the component that owns the listener: once
/// the listener is dropped the callback is not called again, not even for events that were queued
/// while it was running.
///
/// Events are handled run-to-completion: if the callback provokes another event for the same
/// listener (e.g. by writing the DOM), it is queued and delivered once the callback returns,
/// instead of re-entering it.
pub struct Listener {
	target: EventTarget,
	name: &'static str,
	closure: Closure<dyn Fn(Event)>,
	closed: Rc<Cell<bool>>,
}

struct Dispatch<F> {
	callback: RefCell<F>,
	pending: RefCell<VecDeque<Event>>,
	closed: Rc<Cell<bool>>,
}

impl<F: FnMut(Event)> Dispatch<F> {
	fn dispatch(&self, event: Event) {
		if self.closed.get() {
			return;
		}
		let Ok(mut callback) = self.callback.try_borrow_mut() else {
			self.pending.borrow_mut().push_back(event);
			return;
		};
		callback(event);
		loop {
			// the callback may have dropped its own listener
			if self.closed.get() {
				self.pending.borrow_mut().clear();
				break;
			}
			let next = self.pending.borrow_mut().pop_front();
			let Some(event) = next else { break };
			callback(event);
		}
	}
}

impl Listener {
	/// Call `callback` for each `name` event on `target` until the returned listener is dropped.
	/// Fails if the browser refuses to add the listener.
	pub fn new(
		target: &EventTarget,
		name: &'static str,
		callback: impl FnMut(Event) + 'static,
	) -> Result<Self, Error> {
		let closed = Rc::new(Cell::new(false));
		let dispatch = Rc::new(Dispatch {
			callback: RefCell::new(callback),
			pending: RefCell::new(VecDeque::new()),
			closed: closed.clone(),
		});
		// `Fn` (not `FnMut`) at the wasm-bindgen level: re-entrancy is handled by `Dispatch`
		let closure = Closure::<dyn Fn(Event)>::new(move |event| dispatch.dispatch(event));
		target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
		Ok(Self { target: target.clone(), name, closure, closed })
	}
}

impl Drop for Listener {
	fn drop(&mut self) {
		self.closed.set(true);
		// a failing removal during teardown has no meaningful recovery
		let _ = self
			.target
			.remove_event_listener_with_callback(self.name, self.closure.as_ref().unchecked_ref());
	}
}
