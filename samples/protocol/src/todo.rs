//! The replicated model of the todo sample, and the actions that change it.
//!
//! The server owns the model. A client never edits its copy: it sends an [`Action`], the server
//! applies it with [`Todos::apply`] and replicates the resulting model to every client.

use serde::{Deserialize, Serialize};

const MAX_TEXT_CHARS: usize = 200;
/// The model is replicated whole to every client after every action, so its size is bounded.
pub const MAX_ITEMS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
	pub id: u32,
	pub text: String,
	pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todos {
	pub items: Vec<Item>,
	/// Server-side bookkeeping, not replicated.
	#[serde(skip)]
	next_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
	Add { text: String },
	Toggle { id: u32 },
	Remove { id: u32 },
	ClearDone,
}

impl Todos {
	/// The reducer: the only way the model changes. Invalid actions (blank text, unknown id, a full
	/// list) are ignored, as a stale client may legitimately send them.
	pub fn apply(&mut self, action: Action) {
		match action {
			Action::Add { text } => {
				let text: String = text.trim().chars().take(MAX_TEXT_CHARS).collect();
				if text.is_empty() || self.items.len() >= MAX_ITEMS {
					return;
				}
				// ids are never reused; running out of them ends the list rather than wrapping
				let Some(id) = self.next_id.checked_add(1) else { return };
				self.next_id = id;
				self.items.push(Item { id, text, done: false });
			}
			Action::Toggle { id } => {
				if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
					item.done = !item.done;
				}
			}
			Action::Remove { id } => self.items.retain(|item| item.id != id),
			Action::ClearDone => self.items.retain(|item| !item.done),
		}
	}

	pub fn done_count(&self) -> usize {
		self.items.iter().filter(|item| item.done).count()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn add(todos: &mut Todos, text: &str) {
		todos.apply(Action::Add { text: text.into() });
	}

	#[test]
	fn add_assigns_increasing_ids_that_are_never_reused() {
		let mut todos = Todos::default();
		add(&mut todos, "a");
		add(&mut todos, "b");
		todos.apply(Action::Remove { id: 2 });
		add(&mut todos, "c");
		let ids: Vec<u32> = todos.items.iter().map(|item| item.id).collect();
		assert_eq!(ids, [1, 3]);
	}

	#[test]
	fn blank_text_is_ignored_and_long_text_is_cut() {
		let mut todos = Todos::default();
		add(&mut todos, "   ");
		assert!(todos.items.is_empty());
		add(&mut todos, &"x".repeat(500));
		assert_eq!(todos.items[0].text.chars().count(), 200);
	}

	#[test]
	fn the_list_is_bounded_and_has_room_again_after_a_removal() {
		let mut todos = Todos::default();
		for index in 0..MAX_ITEMS + 10 {
			add(&mut todos, &format!("item {index}"));
		}
		assert_eq!(todos.items.len(), MAX_ITEMS);
		todos.apply(Action::Remove { id: 1 });
		add(&mut todos, "one more");
		assert_eq!(todos.items.len(), MAX_ITEMS);
		assert_eq!(todos.items.last().unwrap().text, "one more");
	}

	#[test]
	fn running_out_of_ids_stops_adding_instead_of_wrapping() {
		let mut todos = Todos { next_id: u32::MAX, ..Todos::default() };
		add(&mut todos, "no id left");
		assert!(todos.items.is_empty());
	}

	#[test]
	fn toggle_remove_and_clear_done() {
		let mut todos = Todos::default();
		for text in ["a", "b", "c"] {
			add(&mut todos, text);
		}
		todos.apply(Action::Toggle { id: 1 });
		todos.apply(Action::Toggle { id: 3 });
		assert_eq!(todos.done_count(), 2);
		todos.apply(Action::Toggle { id: 3 });
		assert_eq!(todos.done_count(), 1);
		todos.apply(Action::ClearDone);
		assert_eq!(todos.items.len(), 2);
		todos.apply(Action::Remove { id: 2 });
		assert_eq!(todos.items.len(), 1);
	}

	#[test]
	fn unknown_ids_are_ignored() {
		let mut todos = Todos::default();
		add(&mut todos, "a");
		let before = todos.clone();
		todos.apply(Action::Toggle { id: 99 });
		todos.apply(Action::Remove { id: 99 });
		assert_eq!(todos, before);
	}

	#[test]
	fn the_wire_format_is_plain_json_and_hides_server_bookkeeping() {
		let mut todos = Todos::default();
		add(&mut todos, "a");
		assert_eq!(
			serde_json::to_string(&todos).unwrap(),
			r#"{"items":[{"id":1,"text":"a","done":false}]}"#
		);
		assert_eq!(
			serde_json::to_string(&Action::Toggle { id: 7 }).unwrap(),
			r#"{"type":"toggle","id":7}"#
		);
		assert_eq!(
			serde_json::from_str::<Action>(r#"{"type":"clear_done"}"#).unwrap(),
			Action::ClearDone
		);
		assert!(serde_json::from_str::<Action>(r#"{"type":"nope"}"#).is_err());
	}
}
