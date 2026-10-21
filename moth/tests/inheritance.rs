//! `enabled` inheritance: a component is effectively enabled when it and every ancestor are.

mod common;

use std::{cell::Cell, rc::Rc};

use common::{document, inner};
use moth::{Anchor, Body, Component, PushButton};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

fn counting_button(parent: &impl Component) -> (PushButton, Rc<Cell<u32>>) {
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let button = PushButton::new(parent, "b", move || counter.set(counter.get() + 1)).unwrap();
	(button, clicks)
}

fn inner_disabled(button: &PushButton) -> bool {
	inner(button.element(), "button").has_attribute("disabled")
}

#[wasm_bindgen_test]
fn a_disabled_parent_disables_its_children() {
	let body = Body::new().unwrap();
	let (button, clicks) = counting_button(&body);
	assert!(button.effectively_enabled());

	body.set_enabled(false);
	assert!(!button.effectively_enabled());
	assert!(inner_disabled(&button));
	button.element().click();
	assert_eq!(clicks.get(), 0);

	body.set_enabled(true);
	assert!(button.effectively_enabled());
	assert!(!inner_disabled(&button));
	button.element().click();
	assert_eq!(clicks.get(), 1);
}

#[wasm_bindgen_test]
fn own_enabled_and_the_disabled_attribute_stay_the_child_s_own() {
	let body = Body::new().unwrap();
	let (button, _) = counting_button(&body);
	body.set_enabled(false);
	assert!(button.enabled()); // its own property is untouched
	assert!(!button.element().has_attribute("disabled")); // so is its attribute
	assert!(!button.effectively_enabled());
}

#[wasm_bindgen_test]
fn inheritance_crosses_several_levels() {
	let body = Body::new().unwrap();
	let middle: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	body.element().append_child(&middle).unwrap();
	let middle = Anchor::new(&middle).unwrap();
	let (button, clicks) = counting_button(&middle);

	body.set_enabled(false);
	assert!(!middle.effectively_enabled());
	assert!(!button.effectively_enabled());
	button.element().click();
	assert_eq!(clicks.get(), 0);

	// the middle level re-enabled itself, but the root still blocks the whole subtree
	middle.set_enabled(true);
	assert!(!button.effectively_enabled());

	body.set_enabled(true);
	assert!(button.effectively_enabled());
}

#[wasm_bindgen_test]
fn a_child_created_under_a_disabled_parent_starts_disabled() {
	let body = Body::new().unwrap();
	body.set_enabled(false);
	let (button, clicks) = counting_button(&body);
	assert!(!button.effectively_enabled());
	assert!(inner_disabled(&button));
	button.element().click();
	assert_eq!(clicks.get(), 0);
}

#[wasm_bindgen_test]
fn a_native_disabled_fieldset_disables_moth_components_too() {
	let fieldset: HtmlElement = document().create_element("fieldset").unwrap().unchecked_into();
	fieldset.set_attribute("disabled", "").unwrap();
	document().body().unwrap().append_child(&fieldset).unwrap();
	let anchor = Anchor::new(&fieldset).unwrap();
	let (button, clicks) = counting_button(&anchor);

	assert!(!button.effectively_enabled());
	button.element().click();
	assert_eq!(clicks.get(), 0);
	fieldset.remove();
}

#[wasm_bindgen_test]
fn an_adopted_element_reads_its_disabled_attribute() {
	let div: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	div.set_attribute("disabled", "").unwrap();
	let anchor = Anchor::new(&div).unwrap();
	assert!(!anchor.enabled());
	anchor.set_enabled(true);
	assert!(!div.has_attribute("disabled"));
}

#[wasm_bindgen_test]
fn dropping_an_anchor_leaves_its_element_in_the_page() {
	let div: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&div).unwrap();
	let anchor = Anchor::new(&div).unwrap();
	let (button, _) = counting_button(&anchor);
	let button_element = button.element().clone();

	drop(button);
	assert!(!button_element.is_connected()); // the component's own element goes
	drop(anchor);
	assert!(div.is_connected()); // the adopted one does not
	div.remove();
}
