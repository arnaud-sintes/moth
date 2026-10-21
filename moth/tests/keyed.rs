mod common;

use std::{cell::Cell, rc::Rc};

use common::document;
use moth::{Body, Component, ElementRef, Keyed, Label, Panel};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlElement, MutationObserver, MutationObserverInit, MutationRecord};

wasm_bindgen_test_configure!(run_in_browser);

struct Item {
	id: u32,
	text: &'static str,
}

fn item(id: u32, text: &'static str) -> Item {
	Item { id, text }
}

/// A list root and the machinery to sync it.
struct List {
	root: ElementRef,
	rows: Keyed<u32, Label>,
	created: Rc<Cell<u32>>,
	// declared last so that the rows are dropped before their container
	_panel: Panel,
	_body: Body,
}

impl List {
	fn new() -> Self {
		let body = Body::new().unwrap();
		let panel = Panel::new(&body).unwrap();
		// an inert reference to the panel's element, to create the rows under
		let root = ElementRef::new(panel.element());
		Self { root, rows: Keyed::new(), created: Rc::default(), _panel: panel, _body: body }
	}

	fn sync(&mut self, items: &[Item]) -> Result<(), moth::Error> {
		let root = &self.root;
		let created = self.created.clone();
		self.rows.sync(
			root.element(),
			items,
			|item| item.id,
			|item| {
				created.set(created.get() + 1);
				Label::new(root, item.text)
			},
			|label, item| label.set_text(item.text),
		)
	}

	/// The text of every child element (the `Keyed`'s end marker is a comment, not an element).
	fn texts(&self) -> Vec<String> {
		let children = self.root.element().child_nodes();
		(0..children.length())
			.map(|index| children.item(index).unwrap())
			.filter(|node| node.node_type() == web_sys::Node::ELEMENT_NODE)
			.map(|node| node.text_content().unwrap_or_default())
			.collect()
	}

	fn elements(&self, ids: &[u32]) -> Vec<HtmlElement> {
		ids.iter().map(|id| self.rows.get(id).unwrap().element().clone()).collect()
	}
}

#[wasm_bindgen_test]
fn new_keys_get_components_in_the_items_order() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	assert_eq!(list.texts(), ["a", "b", "c"]);
	assert_eq!(list.rows.len(), 3);
}

#[wasm_bindgen_test]
fn an_item_that_changed_keeps_its_component() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b")]).unwrap();
	let before = list.elements(&[1, 2]);

	list.sync(&[item(1, "a"), item(2, "B")]).unwrap();
	assert_eq!(list.texts(), ["a", "B"]);
	assert_eq!(list.created.get(), 2); // nothing was rebuilt
	assert_eq!(list.elements(&[1, 2]), before);
}

#[wasm_bindgen_test]
fn a_removed_item_drops_its_component_and_element() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	let gone = list.elements(&[2])[0].clone();

	list.sync(&[item(1, "a"), item(3, "c")]).unwrap();
	assert!(!gone.is_connected());
	assert_eq!(list.texts(), ["a", "c"]);
	assert!(list.rows.get(&2).is_none());
}

#[wasm_bindgen_test]
fn reordering_moves_the_existing_elements() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	let before = list.elements(&[1, 2, 3]);

	list.sync(&[item(3, "c"), item(1, "a"), item(2, "b")]).unwrap();
	assert_eq!(list.texts(), ["c", "a", "b"]);
	assert_eq!(list.created.get(), 3);
	assert_eq!(list.elements(&[1, 2, 3]), before); // same elements, only moved
}

#[wasm_bindgen_test]
fn an_insertion_in_the_middle_lands_in_the_middle() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(3, "c")]).unwrap();
	list.sync(&[item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	assert_eq!(list.texts(), ["a", "b", "c"]);

	list.sync(&[item(4, "d"), item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	assert_eq!(list.texts(), ["d", "a", "b", "c"]);
}

#[wasm_bindgen_test]
fn adding_removing_and_reordering_in_one_sync() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b"), item(3, "c")]).unwrap();
	list.sync(&[item(3, "c"), item(4, "d"), item(1, "a")]).unwrap();
	assert_eq!(list.texts(), ["c", "d", "a"]);
	assert_eq!(list.rows.len(), 3);
}

#[wasm_bindgen_test]
fn duplicate_keys_are_refused_and_change_nothing() {
	let mut list = List::new();
	list.sync(&[item(1, "a")]).unwrap();
	let error = list.sync(&[item(1, "x"), item(1, "y"), item(2, "z")]).unwrap_err();
	assert!(error.to_string().contains("duplicate"), "{error}");
	assert_eq!(list.texts(), ["a"]);
	assert_eq!(list.rows.len(), 1);
}

#[wasm_bindgen_test]
fn syncing_to_nothing_and_clear_drop_everything() {
	let mut list = List::new();
	list.sync(&[item(1, "a"), item(2, "b")]).unwrap();
	list.sync(&[]).unwrap();
	assert!(list.rows.is_empty());
	assert_eq!(list.texts(), Vec::<String>::new());

	list.sync(&[item(1, "a")]).unwrap();
	list.rows.clear();
	assert_eq!(list.texts(), Vec::<String>::new());
}

/// Counts the elements that leave the parent: moving an element removes it and inserts it again.
struct Moves {
	count: Rc<Cell<u32>>,
	observer: MutationObserver,
	_callback: Closure<dyn FnMut(js_sys::Array)>,
}

impl Moves {
	fn watch(parent: &HtmlElement) -> Self {
		let count = Rc::new(Cell::new(0u32));
		let counter = count.clone();
		let callback = Closure::<dyn FnMut(js_sys::Array)>::new(move |records: js_sys::Array| {
			for record in records.iter() {
				let record: MutationRecord = record.unchecked_into();
				counter.set(counter.get() + record.removed_nodes().length());
			}
		});
		let observer = MutationObserver::new(callback.as_ref().unchecked_ref()).unwrap();
		let options = MutationObserverInit::new();
		options.set_child_list(true);
		observer.observe_with_options(parent, &options).unwrap();
		Self { count, observer, _callback: callback }
	}

	/// The moves so far (the observer delivers its records in a microtask).
	async fn settle(&self) -> u32 {
		common::sleep(10).await;
		self.count.get()
	}
}

impl Drop for Moves {
	fn drop(&mut self) {
		self.observer.disconnect();
	}
}

async fn moves_for(from: &[u32], to: &[u32]) -> u32 {
	let texts = ["", "a", "b", "c", "d", "e"];
	let items = |ids: &[u32]| ids.iter().map(|id| item(*id, texts[*id as usize])).collect::<Vec<_>>();
	let mut list = List::new();
	list.sync(&items(from)).unwrap();
	let moves = Moves::watch(list.root.element());
	list.sync(&items(to)).unwrap();
	assert_eq!(list.texts(), to.iter().map(|id| texts[*id as usize]).collect::<Vec<_>>());
	moves.settle().await
}

#[wasm_bindgen_test]
async fn only_the_elements_that_must_move_are_moved() {
	let start = [1, 2, 3, 4, 5];
	assert_eq!(moves_for(&start, &start).await, 0);
	assert_eq!(moves_for(&start, &[5, 1, 2, 3, 4]).await, 1, "the last row to the front");
	assert_eq!(moves_for(&start, &[2, 3, 4, 5, 1]).await, 1, "the first row to the end");
	assert_eq!(moves_for(&start, &[1, 2, 4, 3, 5]).await, 1, "two neighbours swapped");
	assert_eq!(moves_for(&start, &[5, 4, 3, 2, 1]).await, 4, "reversed: all but one");
}

#[wasm_bindgen_test]
async fn rows_added_and_removed_around_a_stable_core_move_nothing_else() {
	let texts = ["", "a", "b", "c", "d", "e"];
	let items = |ids: &[u32]| ids.iter().map(|id| item(*id, texts[*id as usize])).collect::<Vec<_>>();
	let mut list = List::new();
	list.sync(&items(&[2, 3, 4])).unwrap();
	let moves = Moves::watch(list.root.element());

	list.sync(&items(&[1, 3, 5])).unwrap(); // 2 and 4 go, 1 and 5 come, 3 stays put
	assert_eq!(list.texts(), ["a", "c", "e"]);
	// two removals; the two new rows are put in place (each is one move out of the parent's end)
	assert_eq!(moves.settle().await, 4);
}

#[wasm_bindgen_test]
fn rows_can_be_kept_before_a_node_that_already_exists() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let footer: HtmlElement = document().create_element("footer").unwrap().unchecked_into();
	panel.element().append_child(&footer).unwrap();
	let root = ElementRef::new(panel.element());

	let mut rows = Keyed::<u32, Label>::before(&footer);
	let items = [item(1, "a"), item(2, "b")];
	rows.sync(panel.element(), &items, |i| i.id, |i| Label::new(&root, i.text), |_, _| {}).unwrap();
	assert!(panel.element().last_child().unwrap().is_same_node(Some(&footer)));
	assert_eq!(panel.element().first_element_child().unwrap().text_content().unwrap(), "a");

	drop(rows); // the end marker goes with the Keyed
	assert_eq!(panel.element().child_nodes().length(), 1); // only the footer is left
}

#[wasm_bindgen_test]
fn a_component_dropped_with_its_list_removes_its_element() {
	let mut list = List::new();
	list.sync(&[item(1, "a")]).unwrap();
	let element = list.elements(&[1])[0].clone();
	drop(list);
	assert!(!element.is_connected());
}

#[wasm_bindgen_test]
fn other_siblings_are_left_alone() {
	// not part of the list: a heading the page placed before the rows
	let mut list = List::new();
	let heading: HtmlElement = document().create_element("h1").unwrap().unchecked_into();
	list.root.element().append_child(&heading).unwrap();

	list.sync(&[item(1, "a"), item(2, "b")]).unwrap();
	list.sync(&[item(2, "b"), item(1, "a")]).unwrap();
	assert_eq!(list.texts(), ["", "b", "a"]);
	assert!(heading.is_connected());
}
