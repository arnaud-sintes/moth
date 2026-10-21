mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{ErrorWatch, document, fire, inner};
use moth::{
	Body, Component, Dropdown, ElementRef, Label, Panel, PushButton, ToggleButton, emit_from,
	event_detail, events,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{CustomEvent, CustomEventInit, Event, HtmlElement, HtmlSelectElement};

wasm_bindgen_test_configure!(run_in_browser);

fn collect(into: &Rc<RefCell<Vec<Event>>>) -> impl FnMut(Event) + 'static {
	let into = into.clone();
	move |event| into.borrow_mut().push(event)
}

// ---- the generic mechanism

#[wasm_bindgen_test]
fn a_childs_event_reaches_its_ancestors_with_its_detail_and_its_origin() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let label = Label::new(&panel, "child").unwrap();
	let (at_panel, at_body) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(Vec::new())));
	let _panel_listener = panel.on("x-ping", collect(&at_panel)).unwrap();
	let _body_listener = body.on("x-ping", collect(&at_body)).unwrap();

	label.emit("x-ping", &JsValue::from_f64(42.0)).unwrap();

	for heard in [&at_panel, &at_body] {
		let heard = heard.borrow();
		assert_eq!(heard.len(), 1);
		assert_eq!(event_detail(&heard[0]).and_then(|detail| detail.as_f64()), Some(42.0));
		// the target is the element that raised it, whoever is listening
		assert!(heard[0].target().unwrap().dyn_ref::<HtmlElement>().unwrap().is_same_node(Some(label.element())));
	}
}

#[wasm_bindgen_test]
fn a_component_hears_its_own_events_too() {
	let body = Body::new().unwrap();
	let heard = Rc::new(RefCell::new(Vec::new()));
	let _listener = body.on("x-self", collect(&heard)).unwrap();
	body.emit("x-self", &JsValue::NULL).unwrap();
	assert_eq!(heard.borrow().len(), 1);
}

#[wasm_bindgen_test]
fn an_event_raised_inside_a_shadow_root_crosses_it_because_it_is_composed() {
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	let inside: HtmlElement = document().create_element("span").unwrap().unchecked_into();
	label.element().shadow_root().unwrap().append_child(&inside).unwrap();
	let heard = Rc::new(RefCell::new(Vec::new()));
	let _listener = body.on("x-deep", collect(&heard)).unwrap();

	emit_from(&inside, "x-deep", &JsValue::NULL).unwrap();
	assert_eq!(heard.borrow().len(), 1, "a composed event leaves the shadow root");
	// the page sees it as coming from the host, not from inside the shadow tree
	assert!(heard.borrow()[0].target().unwrap().dyn_ref::<HtmlElement>().unwrap().is_same_node(Some(label.element())));

	// the control: the same event, not composed, stays inside (this is why `emit_from` sets it)
	let init = CustomEventInit::new();
	init.set_bubbles(true);
	inside.dispatch_event(&CustomEvent::new_with_event_init_dict("x-deep", &init).unwrap()).unwrap();
	assert_eq!(heard.borrow().len(), 1, "a non-composed event does not");
}

#[wasm_bindgen_test]
fn listening_stops_when_the_listener_is_dropped() {
	let errors = ErrorWatch::new();
	let body = Body::new().unwrap();
	let heard = Rc::new(RefCell::new(Vec::new()));
	let listener = body.on("x-once", collect(&heard)).unwrap();
	body.emit("x-once", &JsValue::NULL).unwrap();
	drop(listener);
	body.emit("x-once", &JsValue::NULL).unwrap();
	assert_eq!(heard.borrow().len(), 1);
	errors.assert_none();
}

#[wasm_bindgen_test]
fn only_custom_events_have_a_detail() {
	assert!(event_detail(&Event::new("click").unwrap()).is_none());
}

#[wasm_bindgen_test]
fn a_parent_may_drop_the_child_that_is_raising_an_event() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let slot: Rc<RefCell<Option<Label>>> = Rc::default();
	*slot.borrow_mut() = Some(Label::new(&panel, "doomed").unwrap());
	let (handler_slot, handled) = (slot.clone(), Rc::new(Cell::new(0)));
	let counter = handled.clone();
	let _listener = panel
		.on("x-bye", move |_| {
			counter.set(counter.get() + 1);
			let child = handler_slot.borrow_mut().take();
			drop(child);
		})
		.unwrap();

	let element = slot.borrow().as_ref().unwrap().element().clone();
	emit_from(&element, "x-bye", &JsValue::NULL).unwrap();
	assert_eq!(handled.get(), 1);
	assert!(!element.is_connected());
}

#[wasm_bindgen_test]
fn an_element_ref_raises_and_hears_events_without_being_a_component() {
	let body = Body::new().unwrap();
	let reference = ElementRef::new(body.element());
	let heard = Rc::new(RefCell::new(Vec::new()));
	let _listener = reference.on("x-ref", collect(&heard)).unwrap();
	reference.emit("x-ref", &JsValue::from_str("hi")).unwrap();
	assert_eq!(event_detail(&heard.borrow()[0]).and_then(|detail| detail.as_string()).as_deref(), Some("hi"));
}

// ---- what the stock components raise

#[wasm_bindgen_test]
fn a_toggle_tells_its_ancestors_the_new_state() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let toggle = ToggleButton::new(&panel, "t", |_| {}).unwrap();
	let states = Rc::new(RefCell::new(Vec::new()));
	let sink = states.clone();
	let _listener = panel
		.on(events::TOGGLE, move |event| {
			sink.borrow_mut().push(event_detail(&event).and_then(|detail| detail.as_bool()));
		})
		.unwrap();

	toggle.element().click();
	toggle.element().click();
	assert_eq!(*states.borrow(), [Some(true), Some(false)]);

	toggle.set_toggled(true); // from code: not a gesture
	toggle.set_enabled(false);
	toggle.element().click(); // disabled: not a gesture either
	assert_eq!(states.borrow().len(), 2);
}

#[wasm_bindgen_test]
fn a_controlled_toggle_raises_the_state_it_asks_for() {
	let body = Body::new().unwrap();
	let toggle = ToggleButton::controlled(&body, "t", |_| {}).unwrap();
	let states = Rc::new(RefCell::new(Vec::new()));
	let sink = states.clone();
	let _listener = body
		.on(events::TOGGLE, move |event| {
			sink.borrow_mut().push(event_detail(&event).and_then(|detail| detail.as_bool()));
		})
		.unwrap();
	toggle.element().click();
	toggle.element().click(); // nobody answered: it asks for the same thing again
	assert_eq!(*states.borrow(), [Some(true), Some(true)]);
	assert!(!toggle.toggled());
}

#[wasm_bindgen_test]
fn the_toggle_event_comes_before_the_callback_so_ancestors_still_see_the_button_attached() {
	let body = Body::new().unwrap();
	let slot: Rc<RefCell<Option<ToggleButton>>> = Rc::default();
	let log = Rc::new(RefCell::new(Vec::new()));
	let (callback_log, handler_slot) = (log.clone(), slot.clone());
	let toggle = ToggleButton::new(&body, "t", move |_| {
		callback_log.borrow_mut().push("callback");
		let toggle = handler_slot.borrow_mut().take();
		drop(toggle); // the callback removes the button
	})
	.unwrap();
	let element = toggle.element().clone();
	*slot.borrow_mut() = Some(toggle);
	let (event_log, probe) = (log.clone(), element.clone());
	let _listener = body
		.on(events::TOGGLE, move |_| event_log.borrow_mut().push(if probe.is_connected() { "event (attached)" } else { "event (detached)" }))
		.unwrap();

	element.click();
	assert_eq!(*log.borrow(), ["event (attached)", "callback"]);
}

#[wasm_bindgen_test]
fn a_markup_toggle_is_heard_by_the_page() {
	moth::define_all().unwrap();
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(r#"<moth-toggle-button id="m">t</moth-toggle-button>"#);
	let toggle: HtmlElement = page.query_selector("#m").unwrap().unwrap().unchecked_into();
	let heard = Rc::new(RefCell::new(Vec::new()));
	let _listener = ElementRef::new(&page).on(events::TOGGLE, collect(&heard)).unwrap();

	inner(&toggle, "button").unchecked_into::<HtmlElement>().click(); // a real click inside the shadow root
	assert_eq!(heard.borrow().len(), 1);
	assert_eq!(event_detail(&heard.borrow()[0]).and_then(|detail| detail.as_bool()), Some(true));
	assert!(toggle.has_attribute("toggled"));
	page.remove();
}

#[wasm_bindgen_test]
fn a_dropdown_raises_the_choice_and_programmatic_changes_raise_nothing() {
	let body = Body::new().unwrap();
	let dropdown = Dropdown::new(&body, vec!["a".into(), "b".into()], |_| {}).unwrap();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	let _listener = body
		.on(events::SELECT, move |event| {
			let detail = event_detail(&event).unwrap();
			sink.borrow_mut().push(if detail.is_null() { None } else { detail.as_f64() });
		})
		.unwrap();

	dropdown.set_selected(Some(0));
	assert!(seen.borrow().is_empty());

	let select: HtmlSelectElement = inner(dropdown.element(), "select").unchecked_into();
	select.set_selected_index(1);
	fire(&select, "change");
	select.set_selected_index(-1);
	fire(&select, "change");
	assert_eq!(*seen.borrow(), [Some(1.0), None]);
}

#[wasm_bindgen_test]
fn a_push_button_needs_no_event_of_its_own_because_click_is_already_composed() {
	let body = Body::new().unwrap();
	let button = PushButton::new(&body, "b", || {}).unwrap();
	let heard = Rc::new(RefCell::new(Vec::new()));
	let _listener = body.on("click", collect(&heard)).unwrap();
	inner(button.element(), "button").unchecked_into::<HtmlElement>().click();
	assert_eq!(heard.borrow().len(), 1);
}
