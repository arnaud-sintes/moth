//! Regression tests for the findings of the two independent audits.

mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{document, fire, inner};
use moth::{
	Anchor, Body, Component, Declarative, Definition, Edit, ElementRef, Error, Host, Image, Keyed,
	Label, Listener, Panel, ProgressBar, Prop, PushButton, Replica, Slider, ToggleButton,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{Event, FormData, HtmlElement, HtmlFormElement, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

fn counting_button(parent: &impl Component) -> (PushButton, Rc<Cell<u32>>) {
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let button = PushButton::new(parent, "b", move || counter.set(counter.get() + 1)).unwrap();
	(button, clicks)
}

fn form() -> (HtmlFormElement, Anchor) {
	let form: HtmlFormElement = document().create_element("form").unwrap().unchecked_into();
	document().body().unwrap().append_child(&form).unwrap();
	let anchor = Anchor::new(&form).unwrap();
	(form, anchor)
}

fn submitted(form: &HtmlFormElement, name: &str) -> Option<String> {
	FormData::new_with_form(form).unwrap().get(name).as_string()
}

// ---- enabled: stale effective state

#[wasm_bindgen_test]
fn a_fieldset_disabled_at_runtime_blocks_the_handlers_below_it() {
	let fieldset: HtmlElement = document().create_element("fieldset").unwrap().unchecked_into();
	document().body().unwrap().append_child(&fieldset).unwrap();
	let anchor = Anchor::new(&fieldset).unwrap();
	let (button, clicks) = counting_button(&anchor);

	fieldset.set_attribute("disabled", "").unwrap();
	button.element().click();
	assert_eq!(clicks.get(), 0);
	fieldset.remove();
}

#[wasm_bindgen_test]
fn an_external_disabled_attribute_on_a_rust_created_host_is_honoured() {
	let body = Body::new().unwrap();
	let (button, clicks) = counting_button(&body);

	button.element().set_attribute("disabled", "").unwrap();
	assert!(!button.enabled());
	button.element().click();
	assert_eq!(clicks.get(), 0);
	assert!(inner(button.element(), "button").has_attribute("disabled"));

	button.element().remove_attribute("disabled").unwrap();
	assert!(button.enabled());
}

#[wasm_bindgen_test]
fn moving_a_component_between_parents_recomputes_whether_it_is_enabled() {
	let body = Body::new().unwrap();
	let (disabled, enabled) = (Panel::new(&body).unwrap(), Panel::new(&body).unwrap());
	disabled.set_enabled(false);
	let (button, clicks) = counting_button(&disabled);
	assert!(!button.effectively_enabled());

	enabled.element().append_child(button.element()).unwrap(); // a plain DOM move
	assert!(button.effectively_enabled());
	assert!(!inner(button.element(), "button").has_attribute("disabled"));
	button.element().click();
	assert_eq!(clicks.get(), 1);

	disabled.element().append_child(button.element()).unwrap(); // and back
	assert!(!button.effectively_enabled());
	assert!(inner(button.element(), "button").has_attribute("disabled"));
}

#[wasm_bindgen_test]
fn a_control_disabled_through_an_ancestor_is_not_submitted() {
	let (form, root) = form();
	let panel = Panel::new(&root).unwrap();
	let edit = Edit::new(&panel, "bob", |_| {}).unwrap();
	edit.set_name("user").unwrap();
	assert_eq!(submitted(&form, "user").as_deref(), Some("bob"));

	panel.set_enabled(false);
	assert_eq!(submitted(&form, "user"), None);
	panel.set_enabled(true);
	assert_eq!(submitted(&form, "user").as_deref(), Some("bob"));
	form.remove();
}

#[wasm_bindgen_test]
fn programmatic_input_events_are_ignored_while_disabled() {
	let body = Body::new().unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let edit = Edit::new(&body, "", move |text| sink.borrow_mut().push(text)).unwrap();
	edit.set_enabled(false);

	let input: HtmlInputElement = inner(edit.element(), "input").unchecked_into();
	input.set_value("sneaky");
	fire(&input, "input");
	assert!(seen.borrow().is_empty());
}

// ---- text components as parents

#[wasm_bindgen_test]
fn setting_the_text_of_a_component_does_not_wipe_its_child_components() {
	let body = Body::new().unwrap();
	let button = PushButton::new(&body, "Open", || {}).unwrap();
	let icon = Image::new(&button).unwrap();
	assert!(icon.element().is_connected());

	button.set_text("Save");
	assert!(icon.element().is_connected());
	assert!(button.element().text_content().unwrap().contains("Save"));
}

// ---- Slider / ProgressBar invariants

#[wasm_bindgen_test]
fn handles_keep_the_clamping_invariant() {
	let body = Body::new().unwrap();
	let slider = Slider::new(&body, 10.0, |_| {}).unwrap();
	assert!(slider.handle().set_value(250.0));
	assert_eq!(slider.value(), 100.0);

	let bar = ProgressBar::new(&body).unwrap();
	assert!(bar.handle().set_value(Some(9.0)));
	assert_eq!(bar.value(), Some(1.0));

	let (slider_handle, bar_handle) = (slider.handle(), bar.handle());
	drop((slider, bar));
	assert!(!slider_handle.set_value(5.0));
	assert!(!bar_handle.set_value(Some(0.5)));
}

#[wasm_bindgen_test]
fn a_slider_set_through_its_handle_ends_up_on_the_step_grid() {
	let body = Body::new().unwrap();
	let slider = Slider::new(&body, 0.0, |_| {}).unwrap();
	slider.set_step(25.0);
	slider.handle().set_value(60.0); // not on the grid: the input shows its nearest step
	let input: HtmlInputElement = inner(slider.element(), "input").unchecked_into();
	assert_eq!(slider.value(), input.value_as_number());
	assert_eq!(slider.value() % 25.0, 0.0);
}

#[wasm_bindgen_test]
fn the_slider_value_matches_what_the_stepped_input_displays() {
	let body = Body::new().unwrap();
	let slider = Slider::new(&body, 30.0, |_| {}).unwrap();
	slider.set_step(25.0);
	let input: HtmlInputElement = inner(slider.element(), "input").unchecked_into();
	assert_eq!(slider.value(), input.value_as_number());
}

#[wasm_bindgen_test]
fn non_finite_numbers_in_markup_are_ignored_like_html_does() {
	moth::define_all().unwrap();
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(
		r#"<form><moth-slider name="s" max="inf" min="-infinity" value="500"></moth-slider></form>"#,
	);
	let form: HtmlFormElement = page.query_selector("form").unwrap().unwrap().unchecked_into();
	// HTML does not parse "inf": the range stays 0..100, so 500 clamps to 100
	assert_eq!(submitted(&form, "s").as_deref(), Some("100"));
	page.remove();
}

// ---- Keyed with foreign siblings

struct Item(u32);

#[wasm_bindgen_test]
fn rows_stay_before_a_footer_that_follows_them() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let root = Anchor::new(panel.element()).unwrap();
	let mut rows = Keyed::<u32, Label>::new();
	let sync = |rows: &mut Keyed<u32, Label>, ids: &[u32]| {
		let items: Vec<Item> = ids.iter().map(|id| Item(*id)).collect();
		rows.sync(
			panel.element(),
			&items,
			|item| item.0,
			|item| Label::new(&root, &item.0.to_string()),
			|_, _| {},
		)
		.unwrap();
	};
	sync(&mut rows, &[1, 2]);
	let footer: HtmlElement = document().create_element("footer").unwrap().unchecked_into();
	panel.element().append_child(&footer).unwrap();

	sync(&mut rows, &[1, 2, 3]);
	sync(&mut rows, &[3, 1, 2]);
	let last = panel.element().last_child().unwrap();
	assert!(last.is_same_node(Some(&footer)), "the footer must stay last");
}

// ---- Listener: events queued while a callback drops its own listener

#[wasm_bindgen_test]
fn no_event_is_delivered_after_the_listener_was_dropped() {
	// at the Listener level: a component's own weak guards would hide the bug
	let target: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	let slot: Rc<RefCell<Option<Listener>>> = Rc::default();
	let calls = Rc::new(Cell::new(0u32));
	let (handler_slot, counter, handler_target) = (slot.clone(), calls.clone(), target.clone());
	let listener = Listener::new(&target, "ping", move |_| {
		counter.set(counter.get() + 1);
		// queued: this callback is running
		handler_target.dispatch_event(&Event::new("ping").unwrap()).unwrap();
		let listener = handler_slot.borrow_mut().take();
		drop(listener); // the listener goes away with the callback still on the stack
	})
	.unwrap();
	*slot.borrow_mut() = Some(listener);

	target.dispatch_event(&Event::new("ping").unwrap()).unwrap();
	assert_eq!(calls.get(), 1);
}

// ---- Prop / Replica: bindings that can be removed

#[wasm_bindgen_test]
fn a_scoped_binding_stops_when_its_subscription_is_dropped() {
	let prop = Prop::new(0);
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let subscription = prop.bind_scoped(move |value| sink.borrow_mut().push(*value));
	prop.set(1);
	drop(subscription);
	prop.set(2);
	assert_eq!(*seen.borrow(), [0, 1]);

	// a plain binding is for the life of the prop
	let forever = Rc::new(Cell::new(0));
	let counter = forever.clone();
	prop.bind(move |_| counter.set(counter.get() + 1));
	prop.set(3);
	assert_eq!(forever.get(), 2);
}

#[wasm_bindgen_test]
fn dropping_a_subscription_after_its_prop_is_harmless_and_scopes_are_independent() {
	let prop = Prop::new(0);
	let (first, second) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
	let (a, b) = (first.clone(), second.clone());
	let keep = prop.bind_scoped(move |_| a.set(a.get() + 1));
	let release = prop.bind_scoped(move |_| b.set(b.get() + 1));
	drop(release);
	prop.set(1);
	assert_eq!((first.get(), second.get()), (2, 1));
	drop(prop);
	drop(keep); // the prop is gone: nothing to undo, nothing to fail
}

#[wasm_bindgen_test]
fn a_scoped_replica_binding_follows_the_same_rules() {
	let replica = Replica::new((0u32, 0u32));
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let subscription = replica.bind_scoped(|model| model.0, move |value| sink.borrow_mut().push(*value));
	replica.apply((1, 5)); // the selected field changed
	replica.apply((1, 6)); // another one did
	drop(subscription);
	replica.apply((2, 6));
	assert_eq!(*seen.borrow(), [0, 1]);
}

// ---- ToggleButton: controlled mode

#[wasm_bindgen_test]
fn a_controlled_toggle_only_requests_and_the_owner_decides() {
	let body = Body::new().unwrap();
	let requests = Rc::new(RefCell::new(Vec::new()));
	let sink = requests.clone();
	let toggle = ToggleButton::controlled(&body, "t", move |wanted| sink.borrow_mut().push(wanted)).unwrap();

	toggle.element().click();
	assert!(!toggle.toggled(), "a click must not change a controlled toggle");
	assert!(!toggle.element().has_attribute("toggled"));
	toggle.element().click(); // the owner has not answered: it still asks for the same thing
	assert_eq!(*requests.borrow(), [true, true]);

	toggle.set_toggled(true); // the owner decided
	assert!(toggle.element().has_attribute("toggled"));
	toggle.element().click();
	assert_eq!(*requests.borrow(), [true, true, false]);
	assert!(toggle.toggled());
}

#[wasm_bindgen_test]
fn a_disabled_controlled_toggle_requests_nothing() {
	let body = Body::new().unwrap();
	let requests = Rc::new(Cell::new(0));
	let counter = requests.clone();
	let toggle = ToggleButton::controlled(&body, "t", move |_| counter.set(counter.get() + 1)).unwrap();
	toggle.set_enabled(false);
	toggle.element().click();
	assert_eq!(requests.get(), 0);
}

// ---- Parent / ElementRef

#[wasm_bindgen_test]
fn an_element_ref_is_a_parent_that_adds_nothing_to_the_element() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let parent = ElementRef::new(panel.element());
	let label = Label::new(&parent, "child").unwrap();
	assert!(label.element().parent_node().unwrap().is_same_node(Some(panel.element())));

	// the panel's own state is untouched by the reference, and by creating children through it
	panel.set_enabled(false);
	assert!(panel.element().has_attribute("disabled"));
	assert!(!label.effectively_enabled());
	panel.set_enabled(true);
	assert!(label.effectively_enabled());
}

#[wasm_bindgen_test]
fn an_element_ref_by_id_finds_the_element_or_nothing() {
	let div: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	div.set_id("audit-element-ref");
	document().body().unwrap().append_child(&div).unwrap();
	assert!(ElementRef::by_id("audit-element-ref").is_some());
	assert!(ElementRef::by_id("audit-no-such-element").is_none());
	div.remove();
}

// ---- declarative: an attach that moves its own element

thread_local! {
	static MOVER_ATTACHES: Cell<u32> = const { Cell::new(0) };
}

struct Mover {
	host: Host,
}

impl Component for Mover {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Mover {
	const DEFINITION: Definition = Definition {
		tag: "test-mover",
		template: "<slot></slot>",
		css: "",
		attributes: &[],
		form_associated: false,
	};

	fn attach(host: Host) -> Result<Self, Error> {
		MOVER_ATTACHES.set(MOVER_ATTACHES.get() + 1);
		// re-inserting itself reconnects the element while it is still being attached
		let parent = host.element().parent_node().unwrap();
		parent.append_child(host.element())?;
		Ok(Self { host })
	}
}

#[wasm_bindgen_test]
fn an_attach_that_moves_its_element_does_not_create_a_second_instance() {
	moth::define::<Mover>().unwrap();
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html("<test-mover></test-mover>");
	assert_eq!(MOVER_ATTACHES.get(), 1);
	page.remove();
}
