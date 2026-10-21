mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{ErrorWatch, sleep, wait_until};
use moth::Timeout;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn it_runs_once_after_the_delay() {
	let runs = Rc::new(Cell::new(0));
	let counter = runs.clone();
	let timeout = Timeout::after(20, move || counter.set(counter.get() + 1)).unwrap();
	assert_eq!(runs.get(), 0, "not before the delay");
	wait_until("the callback", || runs.get() == 1).await;
	sleep(80).await;
	assert_eq!(runs.get(), 1, "only once");
	drop(timeout); // after it ran: harmless
}

#[wasm_bindgen_test]
async fn dropping_it_first_means_it_never_runs() {
	let errors = ErrorWatch::new();
	let runs = Rc::new(Cell::new(0));
	let counter = runs.clone();
	let timeout = Timeout::after(30, move || counter.set(counter.get() + 1)).unwrap();
	drop(timeout);
	sleep(150).await;
	assert_eq!(runs.get(), 0);
	// a timer that was not cleared would call the dropped closure: only reported by the page
	errors.assert_none();
}

#[wasm_bindgen_test]
async fn it_may_be_dropped_from_inside_its_own_callback() {
	let errors = ErrorWatch::new();
	let slot: Rc<RefCell<Option<Timeout>>> = Rc::default();
	let (done, handler_slot) = (Rc::new(Cell::new(false)), slot.clone());
	let flag = done.clone();
	let timeout = Timeout::after(10, move || {
		let timeout = handler_slot.borrow_mut().take();
		drop(timeout);
		flag.set(true);
	})
	.unwrap();
	*slot.borrow_mut() = Some(timeout);
	wait_until("the callback", || done.get()).await;
	assert!(slot.borrow().is_none());
	errors.assert_none();
}
