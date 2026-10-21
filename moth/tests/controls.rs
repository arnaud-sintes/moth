mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{document, fire, inner};
use moth::{Anchor, Body, Component, Dropdown, Edit, ProgressBar, Slider};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{FormData, HtmlElement, HtmlFormElement, HtmlInputElement, HtmlSelectElement};

wasm_bindgen_test_configure!(run_in_browser);

fn input_of(host: &HtmlElement) -> HtmlInputElement {
	inner(host, "input").unchecked_into()
}

fn select_of(host: &HtmlElement) -> HtmlSelectElement {
	inner(host, "select").unchecked_into()
}

/// A `<form>` in the page and a component root over it.
fn form() -> (HtmlFormElement, Anchor) {
	let form: HtmlFormElement = document().create_element("form").unwrap().unchecked_into();
	document().body().unwrap().append_child(&form).unwrap();
	let anchor = Anchor::new(&form).unwrap();
	(form, anchor)
}

fn submitted(form: &HtmlFormElement, name: &str) -> Option<String> {
	FormData::new_with_form(form).unwrap().get(name).as_string()
}

// ---- Edit

#[wasm_bindgen_test]
fn edit_text_is_a_property_bound_to_the_input() {
	let body = Body::new().unwrap();
	let calls = Rc::new(Cell::new(0));
	let counter = calls.clone();
	let edit = Edit::new(&body, "start", move |_| counter.set(counter.get() + 1)).unwrap();
	assert_eq!(input_of(edit.element()).value(), "start");

	edit.set_text("from code");
	assert_eq!(input_of(edit.element()).value(), "from code");
	assert_eq!(calls.get(), 0); // not a user edit

	edit.set_placeholder("type here");
	assert_eq!(input_of(edit.element()).placeholder(), "type here");
}

#[wasm_bindgen_test]
fn edit_reports_user_edits_and_updates_its_property() {
	let body = Body::new().unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let edit = Edit::new(&body, "", move |text| sink.borrow_mut().push(text)).unwrap();

	let input = input_of(edit.element());
	input.set_value("abc");
	fire(&input, "input");

	assert_eq!(edit.text(), "abc");
	assert_eq!(*seen.borrow(), ["abc"]);
}

#[wasm_bindgen_test]
fn edit_is_disabled_through_its_host_and_ancestors() {
	let body = Body::new().unwrap();
	let edit = Edit::new(&body, "", |_| {}).unwrap();
	assert!(!input_of(edit.element()).disabled());
	body.set_enabled(false);
	assert!(input_of(edit.element()).disabled());
	body.set_enabled(true);
	edit.set_enabled(false);
	assert!(input_of(edit.element()).disabled());
}

#[wasm_bindgen_test]
fn edit_submits_its_text_with_the_enclosing_form() {
	let (form, root) = form();
	let edit = Edit::new(&root, "bob", |_| {}).unwrap();
	edit.set_name("user").unwrap();
	assert_eq!(submitted(&form, "user").as_deref(), Some("bob"));

	let input = input_of(edit.element());
	input.set_value("alice");
	fire(&input, "input");
	assert_eq!(submitted(&form, "user").as_deref(), Some("alice"));

	edit.set_enabled(false); // a disabled form control is not submitted
	assert_eq!(submitted(&form, "user"), None);
	form.remove();
}

#[wasm_bindgen_test]
fn dropping_a_form_control_removes_it_from_the_submission() {
	let (form, root) = form();
	let edit = Edit::new(&root, "bob", |_| {}).unwrap();
	edit.set_name("user").unwrap();
	drop(edit);
	assert_eq!(submitted(&form, "user"), None);
	form.remove();
}

// ---- Slider

#[wasm_bindgen_test]
fn slider_clamps_its_value_to_its_range() {
	let body = Body::new().unwrap();
	let slider = Slider::new(&body, 30.0, |_| {}).unwrap();
	assert_eq!(slider.value(), 30.0);
	assert_eq!(input_of(slider.element()).value(), "30");

	slider.set_value(250.0);
	assert_eq!(slider.value(), 100.0);
	slider.set_value(-5.0);
	assert_eq!(slider.value(), 0.0);

	slider.set_value(80.0);
	slider.set_max(60.0); // the range shrinks under the value
	assert_eq!(slider.value(), 60.0);
	assert_eq!(input_of(slider.element()).value(), "60");
	slider.set_min(70.0); // min above max: the value follows min, no panic
	assert_eq!(slider.value(), 70.0);
}

#[wasm_bindgen_test]
fn slider_range_and_step_are_bound_to_the_input() {
	let body = Body::new().unwrap();
	let slider = Slider::new(&body, 0.0, |_| {}).unwrap();
	slider.set_max(10.0);
	slider.set_step(0.5);
	let input = input_of(slider.element());
	assert_eq!(input.get_attribute("max").as_deref(), Some("10"));
	assert_eq!(input.get_attribute("step").as_deref(), Some("0.5"));
}

#[wasm_bindgen_test]
fn slider_reports_user_moves_but_not_programmatic_ones() {
	let body = Body::new().unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let slider = Slider::new(&body, 0.0, move |value| sink.borrow_mut().push(value)).unwrap();

	slider.set_value(10.0);
	assert!(seen.borrow().is_empty());

	let input = input_of(slider.element());
	input.set_value("42");
	fire(&input, "input");
	assert_eq!(*seen.borrow(), [42.0]);
	assert_eq!(slider.value(), 42.0);
}

#[wasm_bindgen_test]
fn slider_submits_its_value() {
	let (form, root) = form();
	let slider = Slider::new(&root, 25.0, |_| {}).unwrap();
	slider.set_name("level").unwrap();
	assert_eq!(submitted(&form, "level").as_deref(), Some("25"));
	slider.set_value(75.0);
	assert_eq!(submitted(&form, "level").as_deref(), Some("75"));
	form.remove();
}

// ---- Dropdown

fn options(texts: &[&str]) -> Vec<String> {
	texts.iter().map(|text| (*text).to_owned()).collect()
}

#[wasm_bindgen_test]
fn dropdown_renders_its_options_and_selection() {
	let body = Body::new().unwrap();
	let dropdown = Dropdown::new(&body, options(&["red", "green", "blue"]), |_| {}).unwrap();
	let select = select_of(dropdown.element());
	assert_eq!(select.length(), 3);
	assert_eq!(dropdown.selected(), None);
	assert_eq!(select.selected_index(), -1);

	dropdown.set_selected(Some(1));
	assert_eq!(select.selected_index(), 1);
	assert_eq!(dropdown.selected_text().as_deref(), Some("green"));

	dropdown.set_selected(Some(9)); // no such option: clears
	assert_eq!(dropdown.selected(), None);
	assert_eq!(select.selected_index(), -1);
}

#[wasm_bindgen_test]
fn dropdown_keeps_its_selection_across_set_options_when_it_still_exists() {
	let body = Body::new().unwrap();
	let dropdown = Dropdown::new(&body, options(&["a", "b", "c"]), |_| {}).unwrap();
	dropdown.set_selected(Some(1));

	dropdown.set_options(options(&["x", "y"]));
	assert_eq!(dropdown.selected(), Some(1));
	assert_eq!(select_of(dropdown.element()).length(), 2);

	dropdown.set_options(options(&["only"]));
	assert_eq!(dropdown.selected(), None);
	assert_eq!(select_of(dropdown.element()).selected_index(), -1);
}

#[wasm_bindgen_test]
fn dropdown_reports_user_choices_but_not_programmatic_ones() {
	let body = Body::new().unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let dropdown =
		Dropdown::new(&body, options(&["a", "b"]), move |index| sink.borrow_mut().push(index))
			.unwrap();

	dropdown.set_selected(Some(0));
	assert!(seen.borrow().is_empty());

	let select = select_of(dropdown.element());
	select.set_selected_index(1);
	fire(&select, "change");
	assert_eq!(*seen.borrow(), [Some(1)]);
	assert_eq!(dropdown.selected(), Some(1));
}

#[wasm_bindgen_test]
fn dropdown_submits_the_selected_text() {
	let (form, root) = form();
	let dropdown = Dropdown::new(&root, options(&["red", "green"]), |_| {}).unwrap();
	dropdown.set_name("color").unwrap();
	assert_eq!(submitted(&form, "color"), None); // nothing selected, nothing submitted

	dropdown.set_selected(Some(1));
	assert_eq!(submitted(&form, "color").as_deref(), Some("green"));
	form.remove();
}

// ---- ProgressBar

#[wasm_bindgen_test]
fn progress_bar_is_indeterminate_until_it_has_a_value() {
	let body = Body::new().unwrap();
	let bar = ProgressBar::new(&body).unwrap();
	let progress = inner(bar.element(), "progress");
	assert!(!progress.has_attribute("value"));

	bar.set_value(Some(0.25));
	assert_eq!(progress.get_attribute("value").as_deref(), Some("0.25"));
	bar.set_value(None);
	assert!(!progress.has_attribute("value"));
}

#[wasm_bindgen_test]
fn progress_bar_clamps_to_its_bounds_and_rejects_a_bad_max() {
	let body = Body::new().unwrap();
	let bar = ProgressBar::new(&body).unwrap();
	bar.set_value(Some(7.0));
	assert_eq!(bar.value(), Some(1.0));
	bar.set_value(Some(-1.0));
	assert_eq!(bar.value(), Some(0.0));

	bar.set_max(200.0).unwrap();
	bar.set_value(Some(150.0));
	assert_eq!(bar.value(), Some(150.0));
	bar.set_max(100.0).unwrap(); // the bound drops under the value
	assert_eq!(bar.value(), Some(100.0));

	assert!(bar.set_max(0.0).is_err());
	assert!(bar.set_max(f64::NAN).is_err());
	assert!(bar.set_max(f64::INFINITY).is_err());
	assert_eq!(bar.max(), 100.0);
}
