//! Run-to-completion semantics for props and listeners.

use std::{
	cell::{Cell, RefCell},
	rc::Rc,
};

use moth::{Body, Component, Listener, Prop, PushButton};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::Event;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn bind_applies_the_current_value_immediately() {
	let prop = Prop::new(7);
	let seen = Rc::new(Cell::new(0));
	let sink = seen.clone();
	prop.bind(move |value| sink.set(*value));
	assert_eq!(seen.get(), 7);
	prop.set(8);
	assert_eq!(seen.get(), 8);
}

#[wasm_bindgen_test]
fn a_set_from_inside_a_binding_is_queued_not_nested() {
	let prop = Prop::new(0);
	let weak = prop.downgrade();
	let first = Rc::new(RefCell::new(Vec::new()));
	let second = Rc::new(RefCell::new(Vec::new()));

	let log = first.clone();
	prop.bind(move |value| {
		log.borrow_mut().push(*value);
		if *value == 1 {
			assert!(weak.set(2));
		}
	});
	let log = second.clone();
	prop.bind(move |value| log.borrow_mut().push(*value));

	prop.set(1);

	// nested application would have shown the second binding "2" before "1"
	assert_eq!(*first.borrow(), vec![0, 1, 2]);
	assert_eq!(*second.borrow(), vec![0, 1, 2]);
	assert_eq!(prop.get(), 2);
}

#[wasm_bindgen_test]
fn get_works_from_inside_a_binding() {
	let prop = Prop::new(String::from("a"));
	let weak = prop.downgrade();
	let seen = Rc::new(RefCell::new(None));
	let sink = seen.clone();
	prop.bind(move |_| *sink.borrow_mut() = weak.get());
	prop.set("b".into());
	assert_eq!(seen.borrow().as_deref(), Some("b"));
}

#[wasm_bindgen_test]
fn a_listener_provoking_its_own_event_is_queued_not_reentered() {
	let body = Body::new().unwrap();
	let log = Rc::new(RefCell::new(Vec::<String>::new()));
	let target = body.element().clone();
	let mut depth = 0;
	let handler_log = log.clone();
	let handler_target = target.clone();
	let _listener = Listener::new(&target, "click", move |_| {
		depth += 1;
		let call = depth;
		handler_log.borrow_mut().push(format!("enter {call}"));
		if call == 1 {
			// fires synchronously while we are running; not `click()`, which the HTML spec
			// turns into a no-op while a click is already in progress on the element
			handler_target.dispatch_event(&Event::new("click").unwrap()).unwrap();
		}
		handler_log.borrow_mut().push(format!("exit {call}"));
	})
	.unwrap();

	target.click();

	assert_eq!(*log.borrow(), ["enter 1", "exit 1", "enter 2", "exit 2"]);
}

#[wasm_bindgen_test]
fn setting_enabled_from_a_click_handler_is_safe() {
	// the handler mutates a prop of the very component whose listener is running
	let body = Body::new().unwrap();
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let slot: Rc<RefCell<Option<moth::WeakProp<bool>>>> = Rc::default();
	let handler_slot = slot.clone();
	let button = PushButton::new(&body, "once", move || {
		counter.set(counter.get() + 1);
		if let Some(enabled) = handler_slot.borrow().as_ref() {
			enabled.set(false);
		}
	})
	.unwrap();
	*slot.borrow_mut() = Some(button.enabled_handle());

	button.element().click();
	button.element().click();

	assert_eq!(clicks.get(), 1);
	assert!(!button.enabled());
}
