use std::{cell::Cell, rc::Rc};

use moth::{Body, Children, Component, Label};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

/// Counts how many instances were dropped.
struct Probe(Rc<Cell<u32>>);

impl Drop for Probe {
	fn drop(&mut self) {
		self.0.set(self.0.get() + 1);
	}
}

#[wasm_bindgen_test]
fn insert_get_remove() {
	let mut children = Children::new();
	let a = children.insert("a");
	let b = children.insert("b");
	assert_eq!(children.len(), 2);
	assert_eq!(children.get(a), Some(&"a"));
	assert_eq!(children.remove(a), Some("a"));
	assert_eq!(children.get(a), None);
	assert_eq!(children.get(b), Some(&"b"));
	assert_eq!(children.len(), 1);
}

#[wasm_bindgen_test]
fn a_stale_key_never_reaches_the_child_that_reused_its_slot() {
	let mut children = Children::new();
	let old = children.insert("old");
	children.remove(old);
	let new = children.insert("new"); // reuses the slot
	assert_ne!(old, new);
	assert_eq!(children.get(old), None);
	assert_eq!(children.remove(old), None);
	assert_eq!(children.get(new), Some(&"new"));
}

#[wasm_bindgen_test]
fn removing_twice_drops_once() {
	let drops = Rc::new(Cell::new(0));
	let mut children = Children::new();
	let key = children.insert(Probe(drops.clone()));
	drop(children.remove(key));
	assert_eq!(drops.get(), 1);
	assert!(children.remove(key).is_none());
	assert_eq!(drops.get(), 1);
}

#[wasm_bindgen_test]
fn clear_and_drop_destroy_every_child_exactly_once() {
	let drops = Rc::new(Cell::new(0));
	let mut children = Children::new();
	let keys: Vec<_> = (0..3).map(|_| children.insert(Probe(drops.clone()))).collect();
	children.clear();
	assert_eq!(drops.get(), 3);
	assert!(children.is_empty());
	assert!(keys.iter().all(|key| !children.contains(*key)));

	children.insert(Probe(drops.clone()));
	drop(children);
	assert_eq!(drops.get(), 4);
}

#[wasm_bindgen_test]
fn iter_visits_live_children_only() {
	let mut children = Children::new();
	let a = children.insert(1);
	children.insert(2);
	children.insert(3);
	children.remove(a);
	let mut values: Vec<_> = children.iter().map(|(_, value)| *value).collect();
	values.sort();
	assert_eq!(values, [2, 3]);
}

#[wasm_bindgen_test]
fn removing_a_component_child_removes_its_element() {
	let body = Body::new().unwrap();
	let mut rows = Children::new();
	let first = rows.insert(Label::new(&body, "one").unwrap());
	rows.insert(Label::new(&body, "two").unwrap());
	assert_eq!(body.element().child_element_count(), 2);

	let element = rows.get(first).unwrap().element().clone();
	drop(rows.remove(first));
	assert!(!element.is_connected());
	assert_eq!(body.element().child_element_count(), 1);

	rows.clear();
	assert_eq!(body.element().child_element_count(), 0);
}
