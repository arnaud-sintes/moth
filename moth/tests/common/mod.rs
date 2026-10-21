#![allow(dead_code)]

use std::{cell::Cell, rc::Rc};

use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, Element, Event, EventInit, EventTarget, HtmlElement};

/// Counts the uncaught errors the page reports while it is alive.
///
/// Calling a wasm-bindgen closure that was dropped throws inside the browser's event loop, where it
/// is only *reported* (an `error` event on the window): the test would go on and pass. So a test
/// that claims "this listener / frame callback / socket handler is gone" must also assert that
/// nothing was reported, or it cannot fail when the unregistration is missing.
pub struct ErrorWatch {
	count: Rc<Cell<u32>>,
	callback: Closure<dyn FnMut(Event)>,
}

impl ErrorWatch {
	pub fn new() -> Self {
		let count = Rc::new(Cell::new(0));
		let counter = count.clone();
		let callback = Closure::<dyn FnMut(Event)>::new(move |_| counter.set(counter.get() + 1));
		web_sys::window()
			.unwrap()
			.add_event_listener_with_callback("error", callback.as_ref().unchecked_ref())
			.unwrap();
		Self { count, callback }
	}

	pub fn errors(&self) -> u32 {
		self.count.get()
	}

	pub fn assert_none(&self) {
		assert_eq!(self.errors(), 0, "the page reported an uncaught error (a call into a dropped closure?)");
	}
}

impl Drop for ErrorWatch {
	fn drop(&mut self) {
		let _ = web_sys::window()
			.unwrap()
			.remove_event_listener_with_callback("error", self.callback.as_ref().unchecked_ref());
	}
}

pub fn document() -> Document {
	web_sys::window().unwrap().document().unwrap()
}

/// An element inside a component's shadow root.
pub fn inner(host: &HtmlElement, selector: &str) -> Element {
	host.shadow_root().unwrap().query_selector(selector).unwrap().unwrap()
}

/// What the browser does for a user gesture on a control: a bubbling, composed event.
pub fn fire(target: &EventTarget, name: &str) {
	let init = EventInit::new();
	init.set_bubbles(true);
	init.set_composed(true);
	target.dispatch_event(&Event::new_with_event_init_dict(name, &init).unwrap()).unwrap();
}

/// Poll until `condition` holds (3 s at most), so tests wait for what they need instead of for a
/// guessed duration.
pub async fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
	for _ in 0..300 {
		if condition() {
			return;
		}
		sleep(10).await;
	}
	panic!("timed out waiting for: {what}");
}

pub async fn sleep(milliseconds: i32) {
	let promise = js_sys::Promise::new(&mut |resolve, _| {
		web_sys::window()
			.unwrap()
			.set_timeout_with_callback_and_timeout_and_arguments_0(
				resolve.unchecked_ref(),
				milliseconds,
			)
			.unwrap();
	});
	JsFuture::from(promise).await.unwrap();
}
