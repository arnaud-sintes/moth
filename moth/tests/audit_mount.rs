//! Own test binary (own page): `pagehide` tears down every mounted root.

mod common;

use std::{cell::RefCell, rc::Rc};

use common::document;
use moth::{Body, Component, Declarative, Definition, Error, Host, Label};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlElement, PageTransitionEvent, PageTransitionEventInit};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
	static DROPPED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct Probe {
	host: Host,
}

impl Component for Probe {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Probe {
	const DEFINITION: Definition = Definition {
		tag: "test-probe",
		template: "<slot></slot>",
		css: "",
		attributes: &[],
		form_associated: false,
	};

	fn attach(host: Host) -> Result<Self, Error> {
		Ok(Self { host })
	}
}

impl Drop for Probe {
	fn drop(&mut self) {
		DROPPED.with_borrow_mut(|dropped| dropped.push(self.host.element().id()));
	}
}

fn page_hide(persisted: bool) {
	let init = PageTransitionEventInit::new();
	init.set_persisted(persisted);
	let event = PageTransitionEvent::new_with_event_init_dict("pagehide", &init).unwrap();
	web_sys::window().unwrap().dispatch_event(&event).unwrap();
}

#[wasm_bindgen_test]
fn teardown_on_pagehide_spares_a_page_that_may_be_restored() {
	moth::define::<Probe>().unwrap();
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(r#"<test-probe id="markup-probe"></test-probe>"#);

	struct Root {
		_body: Body,
		label: Label,
	}
	let body = Body::new().unwrap();
	let label = Label::new(&body, "mounted").unwrap();
	let element = label.element().clone();
	let root = Root { _body: body, label };
	let _ = &root.label;
	moth::mount(root).unwrap();

	// the page goes into the back/forward cache: it may come back, so nothing is torn down
	page_hide(true);
	assert!(element.is_connected(), "a mounted root must survive a persisted pagehide");
	assert!(DROPPED.with_borrow(|dropped| dropped.is_empty()));

	// the page is really going away: Rust owners and markup components are both destroyed
	page_hide(false);
	assert!(!element.is_connected());
	assert_eq!(DROPPED.with_borrow(|dropped| dropped.clone()), ["markup-probe"]);
	let _ = Rc::new(());
}
