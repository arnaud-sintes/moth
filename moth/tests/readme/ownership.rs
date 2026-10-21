use moth::{Body, Component, Label, Panel};

fn main() {
	let body = Body::new().unwrap();
	let list = Panel::new(&body).unwrap();
	let mut rows: Vec<Label> =
		(1..=3).map(|n| Label::new(&list, &format!("row {n}")).unwrap()).collect();

	// a component removes its own element when it is dropped
	let second = rows[1].element().clone();
	assert!(second.is_connected());
	rows.remove(1);
	assert!(!second.is_connected());

	// ...so dropping the parent first is just as safe: nothing looks anything up by id
	drop(list);
	drop(rows);
}
