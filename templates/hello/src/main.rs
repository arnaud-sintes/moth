use std::{cell::Cell, rc::Rc};

use moth::{Body, Component, Label, Panel, PushButton};

/// Everything the page owns. Dropping it removes it all from the DOM.
struct Page {
	_body: Body,
	_card: Panel,
	_title: Label,
	_count: Label,
	_button: PushButton,
}

fn main() {
	// panics go to the browser console, with their file and line, instead of an opaque "unreachable"
	moth::init();

	let body = Body::new().unwrap();
	let card = Panel::new(&body).unwrap();
	card.element().set_class_name("card");
	let title = Label::new(&card, "Hello from moth").unwrap();
	let count = Label::new(&card, "No clicks yet").unwrap();

	// callbacks capture a non-owning handle to what they change, never the component itself
	let text = count.text_handle();
	let clicks = Rc::new(Cell::new(0));
	let button = PushButton::new(&card, "Click me", move || {
		clicks.set(clicks.get() + 1);
		text.set(format!("Clicked {} times", clicks.get()));
	})
	.unwrap();

	// keep the page alive until the browser tab goes away
	moth::mount(Page { _body: body, _card: card, _title: title, _count: count, _button: button })
		.unwrap();
}
