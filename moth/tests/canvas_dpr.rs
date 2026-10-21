//! Own test binary (own page): the page's `matchMedia` and `devicePixelRatio` are replaced by stand-ins.
//!
//! Why stand-ins: a real change of pixel ratio cannot be produced from a test. DevTools emulation
//! (`Emulation.setDeviceMetricsOverride`) changes `devicePixelRatio` but the browser then does not
//! notify an existing `(resolution: Xdppx)` query (checked in Edge/Chromium), so it cannot exercise
//! the listener. What these tests establish is moth's side of the contract: which queries it
//! registers, that a `change` makes the canvas rescale and arm a query for the new ratio, and that
//! all of it is unregistered when the canvas is dropped. That the browser delivers the event for a
//! real monitor change or zoom is the documented behaviour of this technique (MDN, "Monitoring
//! screen resolution or zoom level changes"); it is not observed here.

mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{inner, wait_until};
use js_sys::{Function, Reflect};
use moth::{Body, Canvas, Component, Size};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

/// Installs the stand-ins; returns the control object (`queries`, `active`, `setRatio`, `fire`).
fn install_fakes() -> JsValue {
	Function::new_no_args(
		r#"
		const control = { queries: [], active: 0, fakes: [] };
		control.setRatio = ratio =>
			Object.defineProperty(window, "devicePixelRatio", { configurable: true, get: () => ratio });
		control.fire = () => control.fakes[control.fakes.length - 1].dispatchEvent(new Event("change"));
		window.matchMedia = query => {
			control.queries.push(query);
			const fake = new EventTarget();
			const add = fake.addEventListener.bind(fake);
			const remove = fake.removeEventListener.bind(fake);
			fake.addEventListener = (type, callback, options) => { control.active++; add(type, callback, options); };
			fake.removeEventListener = (type, callback, options) => { control.active--; remove(type, callback, options); };
			fake.media = query;
			fake.matches = true;
			control.fakes.push(fake);
			return fake;
		};
		control.setRatio(1);
		return control;
		"#,
	)
	.call0(&JsValue::NULL)
	.unwrap()
}

fn read(control: &JsValue, name: &str) -> JsValue {
	Reflect::get(control, &name.into()).unwrap()
}

fn queries(control: &JsValue) -> Vec<String> {
	js_sys::Array::from(&read(control, "queries")).iter().map(|q| q.as_string().unwrap()).collect()
}

fn call(control: &JsValue, name: &str, argument: Option<f64>) {
	let function: Function = read(control, name).unchecked_into();
	match argument {
		Some(argument) => function.call1(control, &argument.into()),
		None => function.call0(control),
	}
	.unwrap();
}

#[wasm_bindgen_test]
async fn the_canvas_rescales_when_the_pixel_ratio_changes_and_arms_a_query_for_the_new_ratio() {
	let control = install_fakes();
	let body = Body::new().unwrap();
	let sizes: Rc<RefCell<Vec<Size>>> = Rc::default();
	let sink = sizes.clone();
	let canvas = Canvas::new(&body, move |_, size| sink.borrow_mut().push(size)).unwrap();
	let style = canvas.element().style();
	style.set_property("width", "100px").unwrap();
	style.set_property("height", "50px").unwrap();
	let backing: HtmlCanvasElement = inner(canvas.element(), "canvas").unchecked_into();

	wait_until("a first draw at ratio 1", || {
		sizes.borrow().last().is_some_and(|size| size.width == 100.0 && size.scale == 1.0)
	})
	.await;
	assert_eq!((backing.width(), backing.height()), (100, 50));
	assert_eq!(queries(&control), ["(resolution: 1dppx)"]);

	call(&control, "setRatio", Some(2.0)); // what moving to another monitor does...
	call(&control, "fire", None); // ...and the browser tells the query registered for ratio 1
	wait_until("a redraw at ratio 2", || sizes.borrow().last().is_some_and(|size| size.scale == 2.0)).await;
	assert_eq!((backing.width(), backing.height()), (200, 100), "the backing store follows the ratio");
	let size = *sizes.borrow().last().unwrap();
	assert_eq!((size.width, size.height), (100.0, 50.0), "while the drawing area stays in CSS pixels");
	assert_eq!(queries(&control), ["(resolution: 1dppx)", "(resolution: 2dppx)"]);

	call(&control, "setRatio", Some(1.5));
	call(&control, "fire", None);
	wait_until("a redraw at ratio 1.5", || sizes.borrow().last().is_some_and(|size| size.scale == 1.5)).await;
	assert_eq!((backing.width(), backing.height()), (150, 75));
	assert_eq!(queries(&control).last().unwrap(), "(resolution: 1.5dppx)");
}

#[wasm_bindgen_test]
async fn only_one_query_is_listened_to_at_a_time_and_none_after_the_drop() {
	let control = install_fakes();
	let body = Body::new().unwrap();
	let draws = Rc::new(Cell::new(0u32));
	let counter = draws.clone();
	let canvas = Canvas::new(&body, move |_, _| counter.set(counter.get() + 1)).unwrap();
	let active = || read(&control, "active").as_f64().unwrap();
	assert_eq!(active(), 1.0);

	for ratio in [2.0, 3.0, 2.0] {
		call(&control, "setRatio", Some(ratio));
		call(&control, "fire", None);
		assert_eq!(active(), 1.0, "the previous query's listener is removed when the next is armed");
	}

	drop(canvas);
	assert_eq!(active(), 0.0, "dropping the canvas unregisters its listener");
	// an event for a query that nobody listens to any more must not reach the dropped canvas
	let before = draws.get();
	call(&control, "setRatio", Some(4.0));
	call(&control, "fire", None);
	common::sleep(100).await;
	assert_eq!(draws.get(), before);
}
