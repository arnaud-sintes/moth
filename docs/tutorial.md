# Tutorial: a shopping list

We build a small application: type what you need, add it to a list, tick things off, remove them, and see a summary. It uses most of what moth offers for an ordinary page: components that own their children, a model the page follows, a list kept in step with the model, a component made of other components, and callbacks that do not keep anything alive. It is about 150 lines, all of them shown below; the complete program is [`moth/tests/guides/tutorial.rs`](../moth/tests/guides/tutorial.rs), which the test suite compiles and runs, and this page must show every line of it.

To follow along, start from [the starting project](getting-started.md#the-starting-project) and replace its `src/main.rs`.

## The idea

The page has **one model**, a list of items, and everything on the page is a function of it. Nothing edits another component: a click changes the model, and whatever shows the model follows. In moth the model is a `Prop`: a value that, when set to something different, pushes it to everything *bound* to it.

## 1. The model

An item, and a handle to the list that callbacks can hold:

```rust
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
```

`Item` needs `Clone` and `PartialEq` because a `Prop` compares the new value with the old one and does nothing when they are equal. `Items` is a `WeakProp`: a handle that can read and set the list but does not keep it alive, which is what a callback should hold (a callback that owned the list, or a component, could form a cycle that nothing frees).

`change` is the one way an item is edited: read the list, change the item, set the list. Setting a list that is not different does nothing.

## 2. A row: a component made of components

Each item is shown by a row with its text, a "done" toggle and a "remove" button. A row is a component of our own, made by composing three stock ones inside a `Panel`:

```rust
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
```

Implementing `Component` is all it takes: it says which host element the component has, here the panel's. From then on a `Row` is a parent like any other (`Label::new(&row, ...)` would work), it has the `enabled` property every component has (disabling a row disables its buttons), and **dropping a `Row` drops its children and removes its element**. There is no cleanup code to write.

Creating it:

```rust
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
```

Two things to notice. The toggle is *controlled*: clicking it does not flip it, it only asks, by calling the callback with the state it would like. The list decides (`change` edits the model) and `show` then displays the answer, so the toggle can never disagree with the model. And both callbacks capture `items.clone()`, a weak handle, and the item's `id`, a plain number: nothing in them points at a component.

## 3. The page owns everything

```rust
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
```

The page is a struct holding everything that must stay alive. The underscore names are for fields nobody reads: they are there to *own*. When the struct is dropped, the fields drop, and with them every component and its element.

## 4. The model, and adding items

The start of `main`: panics to the console, the root, a title, and the model.

```rust
fn main() {
	moth::init();
	let body = Body::new().unwrap();
	let title = Label::new(&body, "Shopping list").unwrap();

	// the model: one value, and everything on the page follows it
	let items: Prop<Vec<Item>> = Prop::new(Vec::new());
```

Adding an item needs the text, which an `Edit` keeps and offers as a handle (`text_handle`); the button reads it and updates the model, then sets the draft back to empty, which empties the input:

```rust
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
```

## 5. The list follows the model

```rust
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
```

`bind` runs the closure once now and again every time the list changes. `Keyed::sync` is the heart of it: it makes the rows match the items by key. An item with a new id gets a row made by the `create` closure; an id that is gone has its row **dropped** (this is where destructors pay off: that is the whole cleanup); an item that is still there has its row updated in place, and rows are moved only if the order requires it, so the text typed into one row, or the focus on its button, survives changes elsewhere in the list. (`ElementRef` is a parent that just names an element: the rows are created under the list's element.)

Removing an item from inside its own row's button callback is fine: the button is dropped while its callback runs, and moth makes that safe.

## 6. A summary that follows the same model

A second binding on the same value shows how many items there are:

```rust
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
```

Nothing connects the summary to the list or to the rows; both follow the model, so they cannot disagree. Finally the page is handed to `mount`, which keeps it alive until the tab goes away.

## 7. Run it, and make it yours

`trunk serve`, open the page, add a few things, tick and remove them. The browser's developer tools show the structure: each component is a `<moth-...>` element with a shadow root.

The rows have the class `row` and the list's panel is a plain element in the page, so ordinary CSS lays them out; the starting project's `style.css` shows how, and [styling](styling.md) explains what reaches inside the components and what does not.

## Where next

- **Your own components with their own template and styles**, and components for pages written in HTML: [writing components](components.md).
- **Keeping the list on a server** so that several browsers share it: [`samples/todo`](../samples/todo) is this same idea, with the model on the server (a `Replica` instead of a `Prop`, actions sent over a `Link`) and the rows kept in step with `Keyed::sync` just as here.
- **A bigger page** with panels, sliders, menus and a canvas, with state shared between browsers: [`samples/viewer`](../samples/viewer).
