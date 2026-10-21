use moth::{Body, Component, ElementRef, Keyed, Label, Panel};

struct Todo {
	id: u32,
	text: &'static str,
}

fn main() {
	let body = Body::new().unwrap();
	let list = Panel::new(&body).unwrap();
	let parent = ElementRef::new(list.element());
	let mut rows: Keyed<u32, Label> = Keyed::new();

	// make the rows match the items: new ids get a row, vanished ids drop theirs, the rest are
	// updated in place and moved only if they must be
	let mut show = |todos: &[Todo]| {
		rows.sync(
			list.element(),
			todos,
			|todo| todo.id,
			|todo| Label::new(&parent, todo.text),
			|row, todo| row.set_text(todo.text),
		)
		.unwrap();
	};
	show(&[Todo { id: 1, text: "milk" }, Todo { id: 2, text: "eggs" }]);
	show(&[Todo { id: 2, text: "free-range eggs" }, Todo { id: 3, text: "bread" }]);

	assert_eq!(list.element().text_content().as_deref(), Some("free-range eggsbread"));
}
