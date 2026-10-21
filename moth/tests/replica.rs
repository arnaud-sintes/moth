use std::{cell::{Cell, RefCell}, rc::Rc};

use moth::Replica;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone, PartialEq, Debug)]
struct Model {
	title: String,
	count: u32,
}

fn model(title: &str, count: u32) -> Model {
	Model { title: title.into(), count }
}

#[wasm_bindgen_test]
fn a_binding_runs_now_and_only_when_its_field_changes() {
	let replica = Replica::new(model("a", 0));
	let titles = Rc::new(RefCell::new(Vec::new()));
	let sink = titles.clone();
	replica.bind(|model| model.title.clone(), move |title| sink.borrow_mut().push(title.clone()));
	assert_eq!(*titles.borrow(), ["a"]);

	replica.apply(model("a", 1)); // another field changed
	assert_eq!(*titles.borrow(), ["a"]);
	replica.apply(model("b", 1));
	replica.apply(model("b", 2));
	assert_eq!(*titles.borrow(), ["a", "b"]);
}

#[wasm_bindgen_test]
fn bindings_are_independent() {
	let replica = Replica::new(model("a", 0));
	let (titles, counts) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
	let (t, c) = (titles.clone(), counts.clone());
	replica.bind(|m| m.title.clone(), move |_| t.set(t.get() + 1));
	replica.bind(|m| m.count, move |_| c.set(c.get() + 1));
	assert_eq!((titles.get(), counts.get()), (1, 1));

	replica.apply(model("a", 5));
	assert_eq!((titles.get(), counts.get()), (1, 2));
	replica.apply(model("z", 5));
	assert_eq!((titles.get(), counts.get()), (2, 2));
}

#[wasm_bindgen_test]
fn a_derived_value_can_be_bound() {
	let replica = Replica::new(model("a", 1));
	let parity = Rc::new(RefCell::new(Vec::new()));
	let sink = parity.clone();
	replica.bind(|m| m.count % 2 == 0, move |even| sink.borrow_mut().push(*even));
	for count in [3, 5, 6, 8, 9] {
		replica.apply(model("a", count));
	}
	assert_eq!(*parity.borrow(), [false, true, false]); // only the changes of parity
}

#[wasm_bindgen_test]
fn applying_an_equal_model_does_nothing() {
	let replica = Replica::new(model("a", 1));
	let runs = Rc::new(Cell::new(0));
	let counter = runs.clone();
	replica.bind(|m| m.clone(), move |_| counter.set(counter.get() + 1));
	replica.apply(model("a", 1));
	assert_eq!(runs.get(), 1);
}

#[wasm_bindgen_test]
fn a_handle_reports_when_the_replica_is_gone() {
	let replica = Replica::new(model("a", 1));
	let handle = replica.handle();
	assert!(handle.apply(model("b", 2)));
	assert_eq!(handle.get(), Some(model("b", 2)));
	assert_eq!(replica.get(), model("b", 2));
	drop(replica);
	assert!(!handle.apply(model("c", 3)));
	assert_eq!(handle.get(), None);
}

#[wasm_bindgen_test]
fn a_binding_may_apply_a_further_model_without_nesting() {
	let replica = Replica::new(model("a", 0));
	let handle = replica.handle();
	let seen = Rc::new(RefCell::new(Vec::new()));
	let sink = seen.clone();
	replica.bind(
		|m| m.count,
		move |count| {
			sink.borrow_mut().push(*count);
			if *count == 1 {
				handle.apply(model("a", 2));
			}
		},
	);
	replica.apply(model("a", 1));
	assert_eq!(*seen.borrow(), [0, 1, 2]);
}
