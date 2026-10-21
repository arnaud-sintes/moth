//! Declarative components: elements written in markup, lifetime controlled by the DOM.

mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{document, fire, inner, sleep};
use js_sys::Reflect;
use moth::{Body, Component, Declarative, Definition, Error, Host, Label, PushButton};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{Event, HtmlElement, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
	// (event, element id): keyed by id because async tests interleave, so global counters would mix
	static LOG: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
}

fn log(event: &'static str, id: &str) {
	LOG.with_borrow_mut(|log| log.push((event, id.to_owned())));
}

fn count(event: &str, id: &str) -> usize {
	LOG.with_borrow(|log| log.iter().filter(|(e, i)| *e == event && i == id).count())
}

/// A user component: a label and a button that Rust builds inside the element it is given.
struct Counter {
	id: String,
	host: Host,
	count: Rc<Cell<u32>>,
	label: Label,
	_button: PushButton,
}

impl Component for Counter {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Counter {
	const DEFINITION: Definition = Definition {
		tag: "test-counter",
		template: "<slot></slot>",
		css: ":host { display: block; }",
		attributes: &["start"],
		form_associated: false,
	};

	fn attach(host: Host) -> Result<Self, Error> {
		let id = host.element().id();
		log("attach", &id);
		let label = Label::new(&host, "0")?;
		let count = Rc::new(Cell::new(0u32));
		let (text, counter) = (label.text_handle(), count.clone());
		let button = PushButton::new(&host, "+", move || {
			counter.set(counter.get() + 1);
			text.set(counter.get().to_string());
		})?;
		Ok(Self { id, host, count, label, _button: button })
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		if let ("start", Some(value)) = (name, value) {
			if let Ok(start) = value.parse() {
				self.count.set(start);
				self.label.set_text(value);
			}
		}
	}
}

impl Drop for Counter {
	fn drop(&mut self) {
		log("drop", &self.id);
	}
}

fn setup() {
	moth::define_all().unwrap();
	moth::define::<Counter>().unwrap();
}

/// Markup parsed into a container that is already in the page, so its elements connect.
fn page(html: &str) -> HtmlElement {
	setup();
	let container: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&container).unwrap();
	container.set_inner_html(html);
	container
}

fn first(container: &HtmlElement, selector: &str) -> HtmlElement {
	container.query_selector(selector).unwrap().unwrap().unchecked_into()
}

fn has_instance(element: &HtmlElement) -> bool {
	Reflect::get(element, &"mothId".into()).unwrap().is_truthy()
}

// ---- stock elements in markup

#[wasm_bindgen_test]
fn a_label_takes_its_text_from_an_attribute() {
	let page = page(r#"<moth-label text="hi"></moth-label>"#);
	let label = first(&page, "moth-label");
	assert_eq!(label.text_content().as_deref(), Some("hi"));
	label.set_attribute("text", "yo").unwrap();
	assert_eq!(label.text_content().as_deref(), Some("yo"));
	page.remove();
}

#[wasm_bindgen_test]
fn a_label_keeps_the_content_it_was_written_with() {
	let page = page("<moth-label>plain <b>bold</b></moth-label>");
	assert_eq!(first(&page, "moth-label").inner_html(), "plain <b>bold</b>");
	page.remove();
}

#[wasm_bindgen_test]
fn a_toggle_button_state_is_its_attribute() {
	let page = page(r#"<moth-toggle-button toggled>t</moth-toggle-button>"#);
	let toggle = first(&page, "moth-toggle-button");
	let pressed = || inner(&toggle, "button").get_attribute("aria-pressed");
	assert_eq!(pressed().as_deref(), Some("true"));

	toggle.click(); // flips, and the attribute follows
	assert!(!toggle.has_attribute("toggled"));
	assert_eq!(pressed().as_deref(), Some("false"));

	toggle.set_attribute("toggled", "").unwrap();
	assert_eq!(pressed().as_deref(), Some("true"));
	toggle.remove_attribute("toggled").unwrap();
	assert_eq!(pressed().as_deref(), Some("false"));
	page.remove();
}

#[wasm_bindgen_test]
fn the_disabled_attribute_is_the_own_enabled_state() {
	let page = page("<moth-push-button disabled>x</moth-push-button>");
	let button = first(&page, "moth-push-button");
	let inner_disabled = || inner(&button, "button").has_attribute("disabled");
	assert!(inner_disabled());
	button.remove_attribute("disabled").unwrap();
	assert!(!inner_disabled());
	button.set_attribute("disabled", "").unwrap();
	assert!(inner_disabled());
	page.remove();
}

#[wasm_bindgen_test]
fn range_attributes_apply_before_the_value_whatever_their_order_in_the_markup() {
	let page = page(
		r#"<moth-slider value="150" max="200" min="10" step="5"></moth-slider>
		<moth-progress-bar value="1.5" max="2"></moth-progress-bar>"#,
	);
	let slider = inner(&first(&page, "moth-slider"), "input").unchecked_into::<HtmlInputElement>();
	assert_eq!(slider.value(), "150");
	assert_eq!(slider.get_attribute("step").as_deref(), Some("5"));
	let progress = inner(&first(&page, "moth-progress-bar"), "progress");
	assert_eq!(progress.get_attribute("value").as_deref(), Some("1.5"));
	assert_eq!(progress.get_attribute("max").as_deref(), Some("2"));
	page.remove();
}

#[wasm_bindgen_test]
fn edit_and_image_attributes_reach_their_inner_elements() {
	let page = page(
		r#"<moth-edit value="hi" placeholder="name"></moth-edit>
		<moth-image src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" alt="pixel"></moth-image>"#,
	);
	let input = inner(&first(&page, "moth-edit"), "input").unchecked_into::<HtmlInputElement>();
	assert_eq!(input.value(), "hi");
	assert_eq!(input.placeholder(), "name");
	let image = inner(&first(&page, "moth-image"), "img");
	assert_eq!(image.get_attribute("alt").as_deref(), Some("pixel"));
	assert!(image.get_attribute("src").unwrap().starts_with("data:image/gif"));
	page.remove();
}

#[wasm_bindgen_test]
fn a_markup_edit_lets_the_page_hear_its_input_events_on_the_host() {
	// in markup there is no Rust callback: the composed `input` event is how the page listens
	let page = page(r#"<moth-edit name="e"></moth-edit>"#);
	let edit = first(&page, "moth-edit");
	let heard = Rc::new(Cell::new(0));
	let counter = heard.clone();
	let callback = Closure::<dyn FnMut(Event)>::new(move |_| counter.set(counter.get() + 1));
	edit.add_event_listener_with_callback("input", callback.as_ref().unchecked_ref()).unwrap();

	let input: HtmlInputElement = inner(&edit, "input").unchecked_into();
	input.set_value("x");
	fire(&input, "input");
	assert_eq!(heard.get(), 1);
	page.remove();
}

#[wasm_bindgen_test]
fn elements_created_by_rust_are_not_instantiated_a_second_time() {
	setup();
	let body = Body::new().unwrap();
	let label = Label::new(&body, "x").unwrap();
	assert!(!has_instance(label.element()));
	assert_eq!(label.element().text_content().as_deref(), Some("x"));
}

// ---- a user component and the DOM-controlled lifetime

#[wasm_bindgen_test]
fn a_custom_component_is_built_inside_its_element_with_its_attributes() {
	let page = page(r#"<test-counter id="t-build" start="5"></test-counter>"#);
	let counter = first(&page, "test-counter");
	assert_eq!(count("attach", "t-build"), 1);
	assert!(has_instance(&counter));
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "5");

	first(&counter, "moth-push-button").click();
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "6");

	counter.set_attribute("start", "40").unwrap();
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "40");
	page.remove();
}

#[wasm_bindgen_test]
async fn removing_the_element_drops_the_component_and_its_children() {
	let page = page(r#"<test-counter id="t-remove"></test-counter>"#);
	let counter = first(&page, "test-counter");
	assert_eq!(counter.child_element_count(), 2); // label + button, built by Rust
	assert_eq!(count("drop", "t-remove"), 0);

	counter.remove();
	sleep(20).await;

	assert_eq!(count("attach", "t-remove"), 1);
	assert_eq!(count("drop", "t-remove"), 1);
	assert_eq!(counter.child_element_count(), 0); // children were torn down with the component
	assert!(!has_instance(&counter));
	counter.set_attribute("start", "9").unwrap(); // no instance left: nothing happens, no panic
	page.remove();
}

#[wasm_bindgen_test]
async fn moving_the_element_keeps_the_component() {
	let page = page(r#"<test-counter id="t-move"></test-counter><div id="elsewhere"></div>"#);
	let counter = first(&page, "test-counter");
	let elsewhere = first(&page, "#elsewhere");

	elsewhere.append_child(&counter).unwrap(); // disconnects then reconnects within one task
	sleep(20).await;

	assert_eq!(count("drop", "t-move"), 0);
	assert_eq!(count("attach", "t-move"), 1);
	first(&counter, "moth-push-button").click();
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "1");
	page.remove();
}

#[wasm_bindgen_test]
async fn an_element_connected_again_gets_a_fresh_component() {
	let page = page(r#"<test-counter id="t-again"></test-counter>"#);
	let counter = first(&page, "test-counter");
	first(&counter, "moth-push-button").click(); // state: 1

	counter.remove();
	sleep(20).await;
	page.append_child(&counter).unwrap();

	assert_eq!(count("attach", "t-again"), 2);
	assert_eq!(count("drop", "t-again"), 1);
	assert_eq!(counter.child_element_count(), 2);
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "0");
	page.remove();
}

#[wasm_bindgen_test]
fn the_disabled_attribute_disables_a_custom_component_and_its_children() {
	let page = page("<test-counter disabled></test-counter>");
	let counter = first(&page, "test-counter");
	let button = first(&counter, "moth-push-button");
	let inner_disabled = || inner(&button, "button").has_attribute("disabled");
	assert!(inner_disabled());
	button.click();
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "0");

	counter.remove_attribute("disabled").unwrap();
	assert!(!inner_disabled());
	button.click();
	assert_eq!(counter.query_selector("moth-label").unwrap().unwrap().text_content().unwrap(), "1");
	page.remove();
}
