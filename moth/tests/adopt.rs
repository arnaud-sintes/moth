//! Behaviour and typed access for elements that are already in the page.

mod common;

use std::{cell::Cell, rc::Rc};

use common::{ErrorWatch, document, inner, sleep};
use moth::{Component, Declarative, Definition, ElementRef, Error, Host, Label, PushButton, Slider};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

/// A user component that can be read from outside.
struct Tally {
	host: Host,
	count: Rc<Cell<u32>>,
}

impl Tally {
	fn count(&self) -> u32 {
		self.count.get()
	}
}

impl Component for Tally {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Tally {
	const DEFINITION: Definition = Definition {
		tag: "adopt-tally",
		template: "<slot></slot>",
		css: "",
		attributes: &["start"],
		form_associated: false,
	};

	fn attach(host: Host) -> Result<Self, Error> {
		Ok(Self { host, count: Rc::default() })
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		if let ("start", Some(value)) = (name, value.and_then(|value| value.parse().ok())) {
			self.count.set(value);
		}
	}
}

fn page(html: &str) -> HtmlElement {
	moth::define_all().unwrap();
	moth::define::<Tally>().unwrap();
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(html);
	page
}

fn find(page: &HtmlElement, selector: &str) -> HtmlElement {
	page.query_selector(selector).unwrap().unwrap().unchecked_into()
}

// ---- typed access to a markup component

#[wasm_bindgen_test]
fn rust_can_read_and_change_a_component_the_page_declared() {
	let page = page(r#"<moth-label id="l" text="declared"></moth-label>"#);
	let label = find(&page, "#l");
	let declared = Label::of(&label).expect("a markup label has a component");
	assert_eq!(declared.with(|label| label.text()).as_deref(), Some("declared"));

	declared.with(|label| label.set_text("changed from Rust")).unwrap();
	assert_eq!(label.text_content().as_deref(), Some("changed from Rust"));
	// the attribute still drives it too
	label.set_attribute("text", "changed from markup").unwrap();
	assert_eq!(declared.with(|label| label.text()).as_deref(), Some("changed from markup"));
	page.remove();
}

#[wasm_bindgen_test]
fn the_value_the_user_sets_on_a_markup_slider_can_be_read_from_rust() {
	let page = page(r#"<moth-slider id="s" min="0" max="10" value="3"></moth-slider>"#);
	let slider = Slider::of(&find(&page, "#s")).unwrap();
	assert_eq!(slider.with(|slider| slider.value()), Some(3.0));
	slider.with(|slider| slider.handle().set_value(99.0)); // clamped like any other
	assert_eq!(slider.with(|slider| slider.value()), Some(10.0));
	page.remove();
}

#[wasm_bindgen_test]
fn a_user_component_is_reachable_the_same_way() {
	let page = page(r#"<adopt-tally id="t" start="7"></adopt-tally>"#);
	let tally = ElementRef::by_id("t").unwrap().declared::<Tally>().unwrap();
	assert_eq!(tally.with(Tally::count), Some(7));
	page.remove();
}

#[wasm_bindgen_test]
fn there_is_no_component_where_there_is_none_to_have() {
	let page = page(
		r#"<moth-label id="l"></moth-label><p id="plain"></p><moth-push-button id="b"></moth-push-button>"#,
	);
	// another type, a plain element
	assert!(PushButton::of(&find(&page, "#l")).is_none());
	assert!(Label::of(&find(&page, "#plain")).is_none());
	assert!(Label::of(&find(&page, "#b")).is_none());
	assert!(PushButton::of(&find(&page, "#b")).is_some());

	// an element Rust made owns its component itself: there is no second one to reach
	let body = moth::Body::new().unwrap();
	let label = Label::new(&body, "mine").unwrap();
	assert!(Label::of(label.element()).is_none());

	// an element that is not in the page yet has no component yet
	let loose: HtmlElement = document().create_element("moth-label").unwrap().unchecked_into();
	assert!(Label::of(&loose).is_none());
	page.append_child(&loose).unwrap();
	assert!(Label::of(&loose).is_some());
	page.remove();
}

#[wasm_bindgen_test]
async fn the_access_ends_with_the_element() {
	let page = page(r#"<moth-label id="l" text="x"></moth-label>"#);
	let label = find(&page, "#l");
	let declared = Label::of(&label).unwrap();
	let clone = declared.clone();
	assert!(declared.is_alive());

	label.remove();
	sleep(20).await; // the DOM decides: a disconnection that lasts destroys the component
	assert!(!declared.is_alive());
	assert!(clone.with(|label| label.text()).is_none());
	assert!(Label::of(&label).is_none());
	page.remove();
}

#[wasm_bindgen_test]
fn the_component_lives_through_a_call_that_removes_its_own_element() {
	let page = page(r#"<moth-label id="l" text="x"></moth-label>"#);
	let label = find(&page, "#l");
	let declared = Label::of(&label).unwrap();
	// removing the element inside `with` does not pull the component from under the closure
	let text = declared.with(|label| {
		label.element().remove();
		label.text()
	});
	assert_eq!(text.as_deref(), Some("x"));
	page.remove();
}

// ---- behaviour for existing elements

#[wasm_bindgen_test]
fn behaviour_attached_to_an_existing_element_lives_as_long_as_its_owner() {
	let errors = ErrorWatch::new();
	let page = page(r#"<button id="save">Save</button>"#);
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let save = ElementRef::by_id("save").unwrap();
	let listener = save.on("click", move |_| counter.set(counter.get() + 1)).unwrap();

	find(&page, "#save").click();
	assert_eq!(clicks.get(), 1);
	drop(listener);
	find(&page, "#save").click();
	assert_eq!(clicks.get(), 1);
	assert!(find(&page, "#save").is_connected(), "the element itself is left alone");
	errors.assert_none();
	page.remove();
}

#[wasm_bindgen_test]
fn a_markup_moth_button_can_be_given_behaviour_from_rust() {
	let page = page(r#"<moth-push-button id="go">Go</moth-push-button>"#);
	let clicks = Rc::new(Cell::new(0));
	let counter = clicks.clone();
	let go = ElementRef::by_id("go").unwrap();
	let _click = go.on("click", move |_| counter.set(counter.get() + 1)).unwrap();

	// a real click on the button inside the shadow root
	inner(go.element(), "button").unchecked_into::<HtmlElement>().click();
	assert_eq!(clicks.get(), 1);
	page.remove();
}

#[wasm_bindgen_test]
fn an_element_ref_reports_whether_the_element_is_effectively_enabled() {
	let page = page(r#"<section id="s"><button id="b">x</button></section>"#);
	let button = ElementRef::by_id("b").unwrap();
	assert!(button.effectively_enabled());
	find(&page, "#s").set_attribute("disabled", "").unwrap();
	assert!(!button.effectively_enabled());
	assert!(ElementRef::by_id("no-such-element").is_none());
	page.remove();
}
