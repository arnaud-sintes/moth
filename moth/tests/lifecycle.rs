//! Lifetime guarantees: each test mirrors a failure mode listed in moth's audit.md.

use std::{
	cell::{Cell, RefCell},
	rc::Rc,
};

mod common;

use common::ErrorWatch;
use moth::{Body, Component, Label, PushButton};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn child_is_attached_under_its_parent() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "hello").unwrap();
	assert!(label.element().is_connected());
	let parent = label.element().parent_node().unwrap();
	assert!(parent.is_same_node(Some(body.element())));
	assert_eq!(label.element().text_content().as_deref(), Some("hello"));
}

#[wasm_bindgen_test]
fn elements_are_custom_elements_with_a_shadow_root() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	assert!(label.element().shadow_root().is_some());
	assert_eq!(label.element().tag_name().to_lowercase(), "moth-label");
}

#[wasm_bindgen_test]
fn dropping_a_child_removes_its_element() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	let element = label.element().clone();
	drop(label);
	assert!(!element.is_connected());
	assert_eq!(body.element().child_element_count(), 0);
}

#[wasm_bindgen_test]
fn dropping_the_parent_removes_the_whole_subtree() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	let (body_el, label_el) = (body.element().clone(), label.element().clone());
	drop(body);
	assert!(!body_el.is_connected());
	assert!(!label_el.is_connected());
	drop(label);
}

#[wasm_bindgen_test]
fn parent_dropped_before_child_does_not_fail() {
	// audit: "if the parent dies first -> dangling deref / JS TypeError"
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	let button = PushButton::new(&body, "b", || {}).unwrap();
	drop(body);
	drop(label);
	drop(button);
}

#[wasm_bindgen_test]
fn listener_is_released_with_the_button() {
	let errors = ErrorWatch::new();
	let clicks = Rc::new(Cell::new(0));
	let body = Body::new().unwrap();
	let counter = clicks.clone();
	let button = PushButton::new(&body, "b", move || counter.set(counter.get() + 1)).unwrap();
	let element = button.element().clone();
	element.click();
	assert_eq!(clicks.get(), 1);
	drop(button);
	element.click();
	assert_eq!(clicks.get(), 1);
	// had the listener stayed registered, the browser would have called the dropped closure: that
	// throws, and is only reported
	errors.assert_none();
}

#[wasm_bindgen_test]
fn weak_callback_is_a_noop_once_its_owner_is_gone() {
	// audit: "callbacks capture raw `this` -> use-after-free"
	struct Owner {
		clicks: Cell<u32>,
	}
	let owner = Rc::new(Owner { clicks: Cell::new(0) });
	let weak = Rc::downgrade(&owner);
	let body = Body::new().unwrap();
	let button = PushButton::new(&body, "b", move || {
		if let Some(owner) = weak.upgrade() {
			owner.clicks.set(owner.clicks.get() + 1);
		}
	})
	.unwrap();
	let element = button.element().clone();
	element.click();
	assert_eq!(owner.clicks.get(), 1);
	drop(owner);
	element.click(); // must neither panic nor touch freed state
}

#[wasm_bindgen_test]
fn a_handler_can_drop_its_own_component() {
	// destructor running inside the component's own callback ("delete this row" button)
	let body = Body::new().unwrap();
	let slot: Rc<RefCell<Option<PushButton>>> = Rc::default();
	let handler_slot = slot.clone();
	let button = PushButton::new(&body, "delete", move || {
		let button = handler_slot.borrow_mut().take();
		drop(button);
	})
	.unwrap();
	let element = button.element().clone();
	*slot.borrow_mut() = Some(button);
	element.click();
	assert!(slot.borrow().is_none());
	assert!(!element.is_connected());
	element.click(); // listener is gone: nothing happens
}

#[wasm_bindgen_test]
fn weak_handles_report_death_instead_of_touching_freed_state() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	let text = label.text_handle();
	let enabled = label.enabled_handle();
	assert!(text.set("alive".into()));
	assert_eq!(text.get().as_deref(), Some("alive"));
	drop(label);
	assert!(!text.set("dead".into()));
	assert_eq!(text.get(), None);
	assert!(!enabled.is_alive());
}
