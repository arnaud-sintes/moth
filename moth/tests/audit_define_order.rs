//! Own test binary (own page): the order in which a tag gets defined matters here.

mod common;

use common::document;
use moth::{Body, Label};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn markup_already_in_the_page_is_upgraded_even_if_rust_created_that_tag_first() {
	// markup, while `moth-label` is still undefined
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(r#"<moth-label id="early" text="a"></moth-label>"#);

	// a Rust constructor defines the tag first: the markup element upgrades, no factory yet
	let body = Body::new().unwrap();
	let _label = Label::new(&body, "x").unwrap();
	// only now is the tag registered for markup
	moth::define_all().unwrap();

	let early: HtmlElement = page.query_selector("#early").unwrap().unwrap().unchecked_into();
	early.set_attribute("text", "b").unwrap();
	assert_eq!(early.text_content().as_deref(), Some("b"));
	page.remove();
}

#[wasm_bindgen_test]
fn a_second_component_type_cannot_take_over_a_registered_tag() {
	struct Impostor {
		host: moth::Host,
	}
	impl moth::Component for Impostor {
		fn host(&self) -> &moth::Host {
			&self.host
		}
	}
	impl moth::Declarative for Impostor {
		const DEFINITION: moth::Definition = moth::Definition {
			tag: "moth-label",
			template: "",
			css: "",
			attributes: &[],
			form_associated: false,
		};
		fn attach(host: moth::Host) -> Result<Self, moth::Error> {
			Ok(Self { host })
		}
	}
	moth::define_all().unwrap();
	assert!(moth::define::<Impostor>().is_err());
	moth::define_all().unwrap(); // the legitimate owner can register again
}
