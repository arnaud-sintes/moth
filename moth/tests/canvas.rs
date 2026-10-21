mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{ErrorWatch, inner, sleep, wait_until};
use moth::{Body, Canvas, CanvasHandle, Component, Size};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

wasm_bindgen_test_configure!(run_in_browser);

fn scale() -> f64 {
	web_sys::window().unwrap().device_pixel_ratio()
}

fn set_css_size(canvas: &Canvas, width: &str, height: &str) {
	let style = canvas.element().style();
	style.set_property("width", width).unwrap();
	style.set_property("height", height).unwrap();
}

fn backing(canvas: &Canvas) -> HtmlCanvasElement {
	inner(canvas.element(), "canvas").unchecked_into()
}

fn context(canvas: &Canvas) -> CanvasRenderingContext2d {
	backing(canvas).get_context("2d").unwrap().unwrap().unchecked_into()
}

/// A canvas that records every draw and fills itself red.
fn recording(body: &Body) -> (Canvas, Rc<RefCell<Vec<Size>>>) {
	let sizes = Rc::new(RefCell::new(Vec::new()));
	let sink = sizes.clone();
	let canvas = Canvas::new(body, move |context, size| {
		context.set_fill_style_str("rgb(255, 0, 0)");
		context.fill_rect(0.0, 0.0, size.width, size.height);
		sink.borrow_mut().push(size);
	})
	.unwrap();
	(canvas, sizes)
}

fn last(sizes: &Rc<RefCell<Vec<Size>>>) -> Option<Size> {
	sizes.borrow().last().copied()
}

#[wasm_bindgen_test]
async fn it_draws_at_its_css_size_with_a_matching_backing_store() {
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "200px", "100px");

	wait_until("a draw at 200x100", || {
		last(&sizes).is_some_and(|size| size.width == 200.0 && size.height == 100.0)
	})
	.await;
	let size = last(&sizes).unwrap();
	assert_eq!(size.scale, scale());
	assert_eq!(backing(&canvas).width(), (200.0 * scale()).round() as u32);
	assert_eq!(backing(&canvas).height(), (100.0 * scale()).round() as u32);
}

#[wasm_bindgen_test]
async fn what_the_callback_draws_ends_up_on_the_canvas() {
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "50px", "50px");
	wait_until("a draw", || last(&sizes).is_some_and(|size| size.width == 50.0)).await;

	let pixel = context(&canvas).get_image_data(10.0, 10.0, 1.0, 1.0).unwrap().data().0;
	assert_eq!(pixel, [255, 0, 0, 255]);
}

#[wasm_bindgen_test]
async fn a_resize_redraws_at_the_new_size() {
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "120px", "60px");
	wait_until("120x60", || last(&sizes).is_some_and(|size| size.width == 120.0)).await;

	set_css_size(&canvas, "300px", "90px");
	wait_until("300x90", || {
		last(&sizes).is_some_and(|size| size.width == 300.0 && size.height == 90.0)
	})
	.await;
}

#[wasm_bindgen_test]
async fn many_invalidations_in_a_frame_draw_once() {
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "80px", "40px");
	wait_until("first draw", || last(&sizes).is_some_and(|size| size.width == 80.0)).await;
	sleep(100).await; // let any follow-up frame settle
	let settled = sizes.borrow().len();

	for _ in 0..10 {
		canvas.invalidate();
	}
	wait_until("a redraw", || sizes.borrow().len() > settled).await;
	sleep(100).await;
	assert_eq!(sizes.borrow().len(), settled + 1);
}

#[wasm_bindgen_test]
async fn a_draw_callback_can_ask_for_the_next_frame() {
	let body = Body::new().unwrap();
	let slot: Rc<RefCell<Option<CanvasHandle>>> = Rc::default();
	let (draws, remaining) = (Rc::new(Cell::new(0u32)), Rc::new(Cell::new(0u32)));
	let (counter, left, handle) = (draws.clone(), remaining.clone(), slot.clone());
	let canvas = Canvas::new(&body, move |_, _| {
		counter.set(counter.get() + 1);
		if left.get() > 0 {
			left.set(left.get() - 1);
			if let Some(handle) = handle.borrow().as_ref() {
				handle.invalidate();
			}
		}
	})
	.unwrap();
	// a fixed size: an auto-width canvas would follow the page, and a layout change caused by another
	// test running at the same time (a scrollbar) would add draws to the exact count asserted below
	set_css_size(&canvas, "100px", "50px");
	*slot.borrow_mut() = Some(canvas.handle());
	wait_until("first draw", || draws.get() >= 1).await;
	sleep(100).await;
	let settled = draws.get();

	remaining.set(3);
	canvas.invalidate();
	wait_until("the animation", || draws.get() >= settled + 4).await;
	sleep(100).await;
	assert_eq!(draws.get(), settled + 4); // the requested draw, then three self-requested ones
}

#[wasm_bindgen_test]
async fn dropping_the_canvas_cancels_the_pending_frame() {
	let errors = ErrorWatch::new();
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "60px", "30px");
	wait_until("first draw", || last(&sizes).is_some_and(|size| size.width == 60.0)).await;
	sleep(100).await;
	let settled = sizes.borrow().len();

	canvas.invalidate();
	drop(canvas); // the frame is scheduled but has not run
	sleep(150).await;
	assert_eq!(sizes.borrow().len(), settled);
	// a frame callback still scheduled would call into a dropped closure, which is only reported
	errors.assert_none();
}

#[wasm_bindgen_test]
async fn a_resize_after_the_drop_draws_nothing() {
	let body = Body::new().unwrap();
	let (canvas, sizes) = recording(&body);
	set_css_size(&canvas, "60px", "30px");
	wait_until("first draw", || last(&sizes).is_some_and(|size| size.width == 60.0)).await;
	sleep(100).await;
	let settled = sizes.borrow().len();
	let element = canvas.element().clone();

	drop(canvas); // the observer is disconnected; the element is gone anyway, so re-attach it
	web_sys::window().unwrap().document().unwrap().body().unwrap().append_child(&element).unwrap();
	element.style().set_property("width", "250px").unwrap();
	sleep(150).await;
	assert_eq!(sizes.borrow().len(), settled);
	element.remove();
}

#[wasm_bindgen_test]
async fn a_handle_reports_when_the_canvas_is_gone() {
	let body = Body::new().unwrap();
	let (canvas, _) = recording(&body);
	let handle = canvas.handle();
	assert!(handle.invalidate());
	drop(canvas);
	assert!(!handle.invalidate());
}

#[wasm_bindgen_test]
async fn a_draw_callback_can_drop_its_own_canvas() {
	let body = Body::new().unwrap();
	let slot: Rc<RefCell<Option<Canvas>>> = Rc::default();
	let (draws, handler_slot) = (Rc::new(Cell::new(0u32)), slot.clone());
	let counter = draws.clone();
	let canvas = Canvas::new(&body, move |_, _| {
		counter.set(counter.get() + 1);
		let canvas = handler_slot.borrow_mut().take();
		drop(canvas);
	})
	.unwrap();
	let element = canvas.element().clone();
	set_css_size(&canvas, "40px", "40px");
	*slot.borrow_mut() = Some(canvas);

	wait_until("the draw that drops the canvas", || draws.get() >= 1).await;
	sleep(100).await;
	assert_eq!(draws.get(), 1);
	assert!(slot.borrow().is_none());
	assert!(!element.is_connected());
}
