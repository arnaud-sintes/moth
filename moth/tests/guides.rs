//! The guides' code is these files: each program is compiled and run here, and the guide must contain
//! it verbatim (the tutorial, which walks through one program, must contain every line of it, in
//! excerpts), so the examples can neither rot nor lie. The claims the styling guide makes about CSS are
//! checked against computed styles.

#![allow(dead_code)]

mod common;

use common::{document, fire, inner};
use moth::Declarative;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlElement, HtmlInputElement, PageTransitionEvent, PageTransitionEventInit};

wasm_bindgen_test_configure!(run_in_browser);

const GETTING_STARTED: &str = include_str!("../../docs/getting-started.md");
const TUTORIAL: &str = include_str!("../../docs/tutorial.md");
const COMPONENTS: &str = include_str!("../../docs/components.md");
const STYLING: &str = include_str!("../../docs/styling.md");

mod tutorial {
	include!("guides/tutorial.rs");
	/// The program's own `main` is private to it, as it reads in the guide.
	pub fn run() {
		main()
	}
}
mod stepper {
	include!("guides/stepper.rs");
	pub fn run() {
		main()
	}
}
mod meter {
	include!("guides/meter.rs");
	pub fn run() {
		main()
	}
}
mod badge {
	include!("guides/badge.rs");
	pub fn run() {
		main()
	}
}
mod starter {
	include!("../../templates/hello/src/main.rs");
	pub fn run() {
		main()
	}
}

// ---- the guides contain their code

/// The fenced code blocks of a language.
fn blocks(markdown: &str, language: &str) -> Vec<String> {
	let fence = format!("```{language}");
	let mut found = Vec::new();
	let mut lines = markdown.lines();
	while let Some(line) = lines.next() {
		if line.trim_end() == fence {
			let mut code = String::new();
			for line in lines.by_ref() {
				if line.starts_with("```") {
					break;
				}
				code.push_str(line);
				code.push('\n');
			}
			found.push(code);
		}
	}
	found
}

/// The non-blank lines, without their indentation.
fn lines_of(code: &str) -> Vec<&str> {
	code.lines().map(str::trim).filter(|line| !line.is_empty()).collect()
}

/// Whether `block` is some consecutive lines of `whole`, indentation aside.
fn is_excerpt(block: &str, whole: &str) -> bool {
	let (block, whole) = (lines_of(block), lines_of(whole));
	!block.is_empty() && whole.windows(block.len()).any(|window| window == block.as_slice())
}

#[wasm_bindgen_test]
fn the_tutorial_shows_the_whole_program_in_excerpts_that_are_really_in_it() {
	let program = include_str!("guides/tutorial.rs");
	let excerpts = blocks(TUTORIAL, "rust");
	assert!(excerpts.len() >= 6, "the tutorial has {} code blocks", excerpts.len());
	for excerpt in &excerpts {
		assert!(is_excerpt(excerpt, program), "not in tests/guides/tutorial.rs:\n{excerpt}");
	}
	// nothing of the program is left out: someone who follows the tutorial ends up with all of it
	let shown: Vec<&str> = excerpts.iter().flat_map(|excerpt| lines_of(excerpt)).collect();
	for line in lines_of(program) {
		assert!(shown.contains(&line), "the tutorial never shows this line of the program: {line}");
	}
}

#[wasm_bindgen_test]
fn the_guides_contain_their_files_verbatim() {
	let contains = |guide: &str, name: &str, code: &str| {
		assert!(guide.contains(code.trim_end()), "{name} is not in the guide verbatim");
	};
	contains(COMPONENTS, "stepper.rs", include_str!("guides/stepper.rs"));
	contains(COMPONENTS, "meter.rs", include_str!("guides/meter.rs"));
	contains(COMPONENTS, "badge.rs", include_str!("guides/badge.rs"));
	contains(COMPONENTS, "badge.html", include_str!("guides/badge.html"));
	contains(STYLING, "theme.css", include_str!("guides/theme.css"));
	contains(GETTING_STARTED, "templates/hello/src/main.rs", include_str!("../../templates/hello/src/main.rs"));
	contains(GETTING_STARTED, "templates/hello/Cargo.toml", include_str!("../../templates/hello/Cargo.toml"));
	contains(GETTING_STARTED, "templates/hello/index.html", include_str!("../../templates/hello/index.html"));
	contains(GETTING_STARTED, "templates/hello/style.css", include_str!("../../templates/hello/style.css"));
}

// ---- what the programs do

/// What the browser does when the tab goes away: the mounted roots and markup components are dropped.
fn close_the_tab() {
	let init = PageTransitionEventInit::new();
	let event = PageTransitionEvent::new_with_event_init_dict("pagehide", &init).unwrap();
	web_sys::window().unwrap().dispatch_event(&event).unwrap();
}

fn find_all(selector: &str) -> Vec<HtmlElement> {
	let found = document().body().unwrap().query_selector_all(selector).unwrap();
	(0..found.length()).filter_map(|index| found.get(index)).map(|node| node.unchecked_into()).collect()
}

fn find(selector: &str) -> Option<HtmlElement> {
	find_all(selector).into_iter().next()
}

fn text(element: &HtmlElement) -> String {
	element.text_content().unwrap_or_default()
}

fn button(label: &str) -> HtmlElement {
	find_all("moth-push-button")
		.into_iter()
		.find(|button| text(button) == label)
		.unwrap_or_else(|| panic!("no button labelled {label}"))
}

#[wasm_bindgen_test]
fn the_starter_counts_clicks_and_goes_with_the_tab() {
	starter::run();
	assert_eq!(text(&find("moth-label:nth-of-type(2)").unwrap()), "No clicks yet");
	button("Click me").click();
	button("Click me").click();
	assert_eq!(text(&find_all("moth-label")[1]), "Clicked 2 times");
	close_the_tab();
	assert!(find("moth-label").is_none() && find("moth-panel").is_none());
}

#[wasm_bindgen_test]
fn the_tutorial_app_adds_toggles_removes_and_summarizes() {
	tutorial::run();
	let summary = || find_all("moth-label").last().map(text).unwrap();
	let rows = || find_all("moth-panel.row");
	let edit = find("moth-edit").unwrap();
	let input: HtmlInputElement = inner(&edit, "input").unchecked_into();
	assert_eq!(summary(), "0 items, 0 done");

	let add = |what: &str| {
		input.set_value(what);
		fire(&input, "input");
		button("Add").click();
	};
	add("milk");
	add("eggs");
	assert_eq!(rows().len(), 2);
	assert_eq!(text(&rows()[0]).trim_end_matches("doneremove"), "milk");
	assert_eq!(summary(), "2 items, 0 done");
	assert_eq!(input.value(), "", "adding empties the input");

	add("   ");
	assert_eq!(rows().len(), 2, "nothing typed, nothing added");

	// ticking the first row: the toggle only asks, the list decides, and the toggle then shows it
	let first = rows()[0].clone();
	let toggle = first.query_selector("moth-toggle-button").unwrap().unwrap().unchecked_into::<HtmlElement>();
	toggle.click();
	assert_eq!(summary(), "2 items, 1 done");
	assert_eq!(inner(&toggle, "button").get_attribute("aria-pressed").as_deref(), Some("true"));

	// removing it
	let remove = first.query_selector("moth-push-button").unwrap().unwrap().unchecked_into::<HtmlElement>();
	remove.click();
	assert_eq!(rows().len(), 1);
	assert!(text(&rows()[0]).starts_with("eggs"));
	assert_eq!(summary(), "1 items, 0 done");

	close_the_tab();
	assert!(find("moth-panel").is_none() && find("moth-edit").is_none(), "dropping the page removes it all");
}

#[wasm_bindgen_test]
fn the_stepper_is_a_component_made_of_components_that_reports_with_an_event() {
	stepper::run();
	let status = || text(&find_all("moth-label")[0]);
	assert_eq!(status(), "quantity: 1");
	button("+").click();
	button("+").click();
	assert_eq!(status(), "quantity: 3");
	button("-").click();
	assert_eq!(status(), "quantity: 2");
	assert_eq!(text(&find_all("moth-label")[1]), "2", "its own number follows its value");
	close_the_tab();
	assert!(find("moth-panel").is_none());
}

#[wasm_bindgen_test]
fn a_stepper_made_by_hand_is_a_parent_and_goes_when_dropped() {
	use moth::{Body, Component, Label};
	let body = Body::new().unwrap();
	let stepper = stepper::Stepper::new(&body, 5).unwrap();
	// a component of ours is a parent like any other
	let note = Label::new(&stepper, "inside").unwrap();
	assert!(stepper.element().contains(Some(note.element())));
	stepper.set_value(9);
	assert_eq!(stepper.value(), 9);
	assert_eq!(text(&find_all("moth-label")[0]), "9");
	drop(note);
	drop(stepper);
	assert!(find(".stepper").is_none(), "dropping the component removes its element and its children's");
	drop(body);
}

#[wasm_bindgen_test]
fn the_meter_is_an_element_of_our_own_made_from_rust() {
	meter::run();
	let element = find("guide-meter").expect("the element was created");
	let fill = inner(&element, "#fill");
	assert_eq!(fill.get_attribute("style").as_deref(), Some("width: 50%"));
	assert!(element.shadow_root().is_some(), "its template is in a shadow root of its own");
	close_the_tab();
	assert!(find("guide-meter").is_none(), "dropped with the page");

	// created again, by another page: the element type is already registered, which is fine
	let body = moth::Body::new().unwrap();
	let again = meter::Meter::new(&body, 2.0).unwrap();
	assert_eq!(again.level(), 1.0, "the level is clamped");
	drop(again);
	assert!(find("guide-meter").is_none());
}

#[wasm_bindgen_test]
fn the_badge_is_an_element_for_markup_that_rust_can_reach() {
	let page: HtmlElement = document().create_element("div").unwrap().unchecked_into();
	document().body().unwrap().append_child(&page).unwrap();
	page.set_inner_html(include_str!("guides/badge.html"));

	badge::run();
	let element: HtmlElement = page.query_selector("#unread").unwrap().unwrap().unchecked_into();
	assert_eq!(inner(&element, "#shown").text_content().as_deref(), Some("3"));
	element.set_attribute("count", "7").unwrap();
	assert_eq!(inner(&element, "#shown").text_content().as_deref(), Some("7"));
	badge::Badge::of(&element).unwrap().with(|badge| badge.set_count(9));
	assert_eq!(inner(&element, "#shown").text_content().as_deref(), Some("9"));
	page.remove();
}

// ---- what the styling guide says

fn color_of(element: &web_sys::Element, property: &str) -> String {
	web_sys::window().unwrap().get_computed_style(element).unwrap().unwrap().get_property_value(property).unwrap()
}

#[wasm_bindgen_test]
fn the_theme_in_the_styling_guide_does_what_the_guide_says() {
	use moth::{Body, Component, Edit, Panel, PushButton};
	let style = document().create_element("style").unwrap();
	style.set_text_content(Some(include_str!("guides/theme.css")));
	document().body().unwrap().append_child(&style).unwrap();

	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let button = PushButton::new(&panel, "plain", || {}).unwrap();
	let dark = PushButton::new(&panel, "dark", || {}).unwrap();
	dark.element().set_class_name("danger");
	let edit = Edit::new(&panel, "", |_| {}).unwrap();

	// the variables set on :root (or on any ancestor) reach the inside of the components
	assert_eq!(color_of(&inner(button.element(), "button"), "border-top-left-radius"), "999px");
	assert_eq!(color_of(panel.element(), "display"), "flex");
	// and a variable set on one element changes that element's inside only
	assert_eq!(color_of(&inner(dark.element(), "button"), "background-color"), "rgb(122, 18, 48)");
	assert_ne!(color_of(&inner(button.element(), "button"), "background-color"), "rgb(122, 18, 48)");
	// ::part() reaches what the variables do not
	assert_eq!(color_of(&inner(edit.element(), "input"), "font-style"), "italic");

	// page CSS that names an inner element does not reach it: it is in the shadow root
	let global = document().create_element("style").unwrap();
	global.set_text_content(Some("button { background: rgb(1, 2, 3) !important; border-radius: 0 !important; }"));
	document().body().unwrap().append_child(&global).unwrap();
	assert_ne!(color_of(&inner(button.element(), "button"), "background-color"), "rgb(1, 2, 3)");
	assert_eq!(color_of(&inner(button.element(), "button"), "border-top-left-radius"), "999px");
	global.remove();

	drop((edit, dark, button, panel, body));
	style.remove();
}
