use std::{cell::Cell, rc::Rc};

use moth::{Body, Label, PushButton};

/// The page owns its components: dropping it removes them from the DOM.
struct Page {
	_body: Body,
	_label: Label,
	_button: PushButton,
}

fn main() {
	moth::init(); // panics go to the browser console, with their file and line
	let body = Body::new().unwrap();
	let label = Label::new(&body, "Hello world").unwrap();

	// callbacks capture a non-owning handle, never the components themselves
	let text = label.text_handle();
	let clicks = Rc::new(Cell::new(0));
	let button = PushButton::new(&body, "Say hi", move || {
		clicks.set(clicks.get() + 1);
		text.set(format!("Hello world x{}", clicks.get()));
	})
	.unwrap();

	// keep the page alive until the browser tab goes away
	moth::mount(Page { _body: body, _label: label, _button: button }).unwrap();
}
