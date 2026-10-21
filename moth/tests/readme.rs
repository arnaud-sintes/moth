//! The README's code is these files: each is compiled and run here, and the README must contain it
//! verbatim, so the examples can neither rot nor lie.

#![allow(dead_code)]

mod common;

use common::document;
use moth::{Declarative, Slider};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlElement, PageTransitionEvent, PageTransitionEventInit};

wasm_bindgen_test_configure!(run_in_browser);

const README: &str = include_str!("../../README.md");

mod hello {
	include!("readme/hello.rs");
	/// The example's own `main` is private to it, as it reads in the README.
	pub fn run() {
		main()
	}
}
mod ownership {
	include!("readme/ownership.rs");
	/// The example's own `main` is private to it, as it reads in the README.
	pub fn run() {
		main()
	}
}
mod events {
	include!("readme/events.rs");
	/// The example's own `main` is private to it, as it reads in the README.
	pub fn run() {
		main()
	}
}
mod list {
	include!("readme/list.rs");
	/// The example's own `main` is private to it, as it reads in the README.
	pub fn run() {
		main()
	}
}
mod markup {
	include!("readme/markup.rs");
	/// The example's own `main` is private to it, as it reads in the README.
	pub fn run() {
		main()
	}
}

#[wasm_bindgen_test]
fn the_readme_contains_every_example_verbatim() {
	for (name, code) in [
		("hello.rs", include_str!("readme/hello.rs")),
		("ownership.rs", include_str!("readme/ownership.rs")),
		("events.rs", include_str!("readme/events.rs")),
		("list.rs", include_str!("readme/list.rs")),
		("markup.rs", include_str!("readme/markup.rs")),
		("markup.html", include_str!("readme/markup.html")),
	] {
		assert!(README.contains(code.trim_end()), "README.md does not contain {name} verbatim");
	}
}

/// What the browser does when the tab goes away: the mounted roots and markup components are dropped.
fn close_the_tab() {
	let init = PageTransitionEventInit::new();
	let event = PageTransitionEvent::new_with_event_init_dict("pagehide", &init).unwrap();
	web_sys::window().unwrap().dispatch_event(&event).unwrap();
}

fn find(selector: &str) -> Option<HtmlElement> {
	document().body().unwrap().query_selector(selector).unwrap().map(|e| e.unchecked_into())
}

#[wasm_bindgen_test]
fn hello_world() {
	hello::run();
	assert_eq!(find("moth-label").unwrap().text_content().as_deref(), Some("Hello world"));
	find("moth-push-button").unwrap().click();
	find("moth-push-button").unwrap().click();
	assert_eq!(find("moth-label").unwrap().text_content().as_deref(), Some("Hello world x2"));

	close_the_tab(); // dropping the page removes everything it made
	assert!(find("moth-label").is_none() && find("moth-push-button").is_none());
}

#[wasm_bindgen_test]
fn ownership() {
	ownership::run(); // its own assertions are the example
	assert!(find("moth-panel").is_none() && find("moth-label").is_none());
}

#[wasm_bindgen_test]
fn events() {
	events::run();
	let status = || find("moth-label").unwrap().text_content();
	assert_eq!(status().as_deref(), Some("off"));
	find("moth-toggle-button").unwrap().click();
	assert_eq!(status().as_deref(), Some("on"));
	find("moth-toggle-button").unwrap().click();
	assert_eq!(status().as_deref(), Some("off"));
	close_the_tab();
}

#[wasm_bindgen_test]
fn list() {
	list::run();
	assert!(find("moth-label").is_none(), "everything is dropped when main returns");
}

#[wasm_bindgen_test]
fn markup() {
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(include_str!("readme/markup.html"));

	markup::run();
	let title = find("#title").unwrap();
	assert_eq!(title.text_content().as_deref(), Some("Sound settings"));

	let volume = Slider::of(&find("#volume").unwrap()).unwrap();
	volume.with(|slider| slider.set_value(8.0));
	assert_eq!(volume.with(|slider| slider.value()), Some(8.0));
	find("#reset").unwrap().click(); // the behaviour the example attached
	assert_eq!(volume.with(|slider| slider.value()), Some(3.0));

	close_the_tab();
	assert!(!volume.is_alive(), "markup components go with the page");
	page.remove();
}
