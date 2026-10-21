use std::{
	cell::{Cell, RefCell},
	rc::Rc,
};

use moth::{
	Body, Component, Edit, ElementRef, Error, Host, Keyed, Label, Panel, Parent, Prop, PushButton,
	ToggleButton, WeakProp,
};

/// One thing to buy.
#[derive(Clone, PartialEq)]
struct Item {
	id: u32,
	text: String,
	done: bool,
}

/// What callbacks hold to reach the list: a handle that does not keep it alive.
type Items = WeakProp<Vec<Item>>;

/// Change the item with this id, wherever the change comes from.
fn change(items: &Items, id: u32, edit: impl FnOnce(&mut Item)) {
	if let Some(mut all) = items.get() {
		if let Some(item) = all.iter_mut().find(|item| item.id == id) {
			edit(item);
		}
		items.set(all);
	}
}

/// A row of the list: a component made of components. To the outside it is its panel, so it can sit
/// in a list and could be the parent of more components.
struct Row {
	panel: Panel,
	label: Label,
	done: ToggleButton,
	_remove: PushButton,
}

impl Component for Row {
	fn host(&self) -> &Host {
		self.panel.host()
	}
}

impl Row {
	fn new(parent: &impl Parent, item: &Item, items: &Items) -> Result<Self, Error> {
		let panel = Panel::new(parent)?;
		panel.element().set_class_name("row");
		let label = Label::new(&panel, &item.text)?;

		let id = item.id;
		let list = items.clone();
		// a controlled toggle only asks: the list decides, and `show` displays the answer
		let done = ToggleButton::controlled(&panel, "done", move |on| {
			change(&list, id, |item| item.done = on)
		})?;

		let list = items.clone();
		let remove = PushButton::new(&panel, "remove", move || {
			if let Some(mut all) = list.get() {
				all.retain(|item| item.id != id);
				list.set(all);
			}
		})?;

		let row = Row { panel, label, done, _remove: remove };
		row.show(item);
		Ok(row)
	}

	/// Bring the row up to date with its item.
	fn show(&self, item: &Item) {
		self.label.set_text(&item.text);
		self.done.set_toggled(item.done);
	}
}

/// Everything the page owns. Dropping it removes it all from the DOM.
struct Page {
	_body: Body,
	_title: Label,
	_input: Edit,
	_add: PushButton,
	_list: Panel,
	_summary: Label,
	_items: Prop<Vec<Item>>,
}

fn main() {
	moth::init();
	let body = Body::new().unwrap();
	let title = Label::new(&body, "Shopping list").unwrap();

	// the model: one value, and everything on the page follows it
	let items: Prop<Vec<Item>> = Prop::new(Vec::new());

	// adding: the input holds what is typed, the button turns it into an item
	let input = Edit::new(&body, "", |_| {}).unwrap();
	input.set_placeholder("what do you need?");
	let draft = input.text_handle();
	let next_id = Rc::new(Cell::new(1));
	let all_items = items.downgrade();
	let add = PushButton::new(&body, "Add", move || {
		let text = draft.get().unwrap_or_default();
		let text = text.trim();
		if text.is_empty() {
			return;
		}
		if let Some(mut all) = all_items.get() {
			all.push(Item { id: next_id.get(), text: text.to_owned(), done: false });
			next_id.set(next_id.get() + 1);
			all_items.set(all);
			draft.set(String::new()); // empties the input
		}
	})
	.unwrap();

	// the list: one row per item, kept in step by key
	let list = Panel::new(&body).unwrap();
	let rows = RefCell::new(Keyed::<u32, Row>::new());
	let parent = ElementRef::new(list.element());
	let list_element = list.element().clone();
	let weak_items = items.downgrade();
	items.bind(move |all| {
		rows.borrow_mut()
			.sync(
				&list_element,
				all,
				|item| item.id,
				|item| Row::new(&parent, item, &weak_items),
				|row, item| row.show(item),
			)
			.unwrap();
	});

	// a summary that follows the same value
	let summary = Label::new(&body, "").unwrap();
	let summary_text = summary.text_handle();
	items.bind(move |all| {
		let done = all.iter().filter(|item| item.done).count();
		summary_text.set(format!("{} items, {} done", all.len(), done));
	});

	moth::mount(Page {
		_body: body,
		_title: title,
		_input: input,
		_add: add,
		_list: list,
		_summary: summary,
		_items: items,
	})
	.unwrap();
}
