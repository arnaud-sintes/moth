use std::{cell::{Cell, RefCell}, rc::Rc};

use moth::{Body, Component, Image, Label, PushButton, ToggleButton};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{Element, HtmlElement};

wasm_bindgen_test_configure!(run_in_browser);

fn inner(host: &HtmlElement, selector: &str) -> Element {
	host.shadow_root().unwrap().query_selector(selector).unwrap().unwrap()
}

#[wasm_bindgen_test]
fn label_text_is_a_property_bound_to_the_dom() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "first").unwrap();
	assert_eq!(label.element().text_content().as_deref(), Some("first"));
	label.set_text("second");
	assert_eq!(label.text(), "second");
	assert_eq!(label.element().text_content().as_deref(), Some("second"));
}

#[wasm_bindgen_test]
fn disabling_a_button_blocks_its_handler_and_reflects_in_the_dom() {
	let body = Body::new().unwrap();
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let button = PushButton::new(&body, "x", move || counter.set(counter.get() + 1)).unwrap();

	button.set_enabled(false);
	assert!(button.element().has_attribute("disabled"));
	assert!(inner(button.element(), "button").has_attribute("disabled"));
	button.element().click();
	assert_eq!(clicks.get(), 0);

	button.set_enabled(true);
	assert!(!button.element().has_attribute("disabled"));
	assert!(!inner(button.element(), "button").has_attribute("disabled"));
	button.element().click();
	assert_eq!(clicks.get(), 1);
}

#[wasm_bindgen_test]
fn a_real_click_on_the_shadow_button_reaches_the_host_handler() {
	let body = Body::new().unwrap();
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let button = PushButton::new(&body, "x", move || counter.set(counter.get() + 1)).unwrap();
	inner(button.element(), "button").unchecked_into::<HtmlElement>().click();
	assert_eq!(clicks.get(), 1);
}

#[wasm_bindgen_test]
fn toggle_button_flips_on_click_and_reports_the_new_state() {
	let body = Body::new().unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let toggle = ToggleButton::new(&body, "t", move |on| sink.borrow_mut().push(on)).unwrap();
	assert!(!toggle.toggled());
	assert!(!toggle.element().has_attribute("toggled"));

	toggle.element().click();
	assert!(toggle.toggled());
	assert!(toggle.element().has_attribute("toggled"));
	assert_eq!(
		inner(toggle.element(), "button").get_attribute("aria-pressed").as_deref(),
		Some("true")
	);

	toggle.element().click();
	assert!(!toggle.toggled());
	assert_eq!(*seen.borrow(), [true, false]);
}

#[wasm_bindgen_test]
fn setting_the_toggle_state_programmatically_does_not_call_the_callback() {
	let body = Body::new().unwrap();
	let calls = Rc::new(Cell::new(0));
	let counter = calls.clone();
	let toggle = ToggleButton::new(&body, "t", move |_| counter.set(counter.get() + 1)).unwrap();
	toggle.set_toggled(true);
	assert!(toggle.element().has_attribute("toggled"));
	assert_eq!(calls.get(), 0);
}

#[wasm_bindgen_test]
fn a_disabled_toggle_button_does_not_flip() {
	let body = Body::new().unwrap();
	let toggle = ToggleButton::new(&body, "t", |_| {}).unwrap();
	toggle.set_enabled(false);
	toggle.element().click();
	assert!(!toggle.toggled());
}

#[wasm_bindgen_test]
fn one_component_can_drive_another_through_weak_handles() {
	let body = Body::new().unwrap();
	let target = PushButton::new(&body, "target", || {}).unwrap();
	let enabled = target.enabled_handle();
	let switch = ToggleButton::new(&body, "switch", move |on| {
		enabled.set(!on);
	})
	.unwrap();
	switch.element().click();
	assert!(!target.enabled());
	drop(target);
	switch.element().click(); // target is gone: a no-op, not a crash
}

#[wasm_bindgen_test]
fn image_source_and_alt_are_bound_to_the_inner_img() {
	let body = Body::new().unwrap();
	let image = Image::new(&body).unwrap();
	let img = inner(image.element(), "img");
	assert!(!img.has_attribute("src"));

	image.set_source("data:image/gif;base64,R0lGODlhAQABAAAAACw=");
	assert_eq!(
		img.get_attribute("src").as_deref(),
		Some("data:image/gif;base64,R0lGODlhAQABAAAAACw=")
	);
	image.set_alt("pixel");
	assert_eq!(img.get_attribute("alt").as_deref(), Some("pixel"));

	image.set_source("");
	assert!(!img.has_attribute("src"));
}
