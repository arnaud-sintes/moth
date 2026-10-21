//! Model replication against the sample back-end's shared todo model (started by
//! scripts/test-wasm.ps1, on 127.0.0.1:2000). The model is shared by every test and client, so each
//! test only looks at items carrying its own unique marker.

mod common;

use std::{cell::{Cell, RefCell}, rc::Rc};

use common::{sleep, wait_until};
use moth::{Link, LinkEvent, Replica, State};
use serde::{Deserialize, Serialize};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

const URL: &str = "ws://127.0.0.1:2000/todo/ws";

#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
struct Model {
	items: Vec<Entry>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
struct Entry {
	id: u32,
	text: String,
	done: bool,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Action {
	Add { text: String },
	Toggle { id: u32 },
	Remove { id: u32 },
	/// Something the server does not know: it must ignore it.
	DoesNotExist,
}

fn connect(replica: &Replica<Model>) -> Link<Model, Action> {
	Link::connect(URL, replica, |_| {}).unwrap()
}

fn add(link: &Link<Model, Action>, text: &str) {
	link.actions().send(&Action::Add { text: text.into() }).unwrap();
}

/// The replica's entries whose text starts with `marker`.
fn mine(replica: &Replica<Model>, marker: &str) -> Vec<Entry> {
	replica.get().items.into_iter().filter(|entry| entry.text.starts_with(marker)).collect()
}

#[wasm_bindgen_test]
async fn actions_change_the_model_and_the_snapshot_comes_back() {
	let replica = Replica::new(Model::default());
	let link = connect(&replica);
	let actions = link.actions();
	wait_until("open", || link.state() == State::Open).await;

	add(&link, "t1 first");
	wait_until("the added item", || mine(&replica, "t1").len() == 1).await;
	let entry = mine(&replica, "t1").remove(0);
	assert!(!entry.done);

	actions.send(&Action::Toggle { id: entry.id }).unwrap();
	wait_until("toggled", || mine(&replica, "t1").first().is_some_and(|e| e.done)).await;

	actions.send(&Action::Remove { id: entry.id }).unwrap();
	wait_until("removed", || mine(&replica, "t1").is_empty()).await;
}

#[wasm_bindgen_test]
async fn every_connected_client_receives_every_change() {
	let (first, second) = (Replica::new(Model::default()), Replica::new(Model::default()));
	let (link1, link2) = (connect(&first), connect(&second));
	wait_until("both open", || link1.state() == State::Open && link2.state() == State::Open).await;

	add(&link1, "t2 shared");
	wait_until("second sees it", || mine(&second, "t2").len() == 1).await;
	assert_eq!(mine(&first, "t2"), mine(&second, "t2"));

	let id = mine(&second, "t2")[0].id;
	link2.actions().send(&Action::Toggle { id }).unwrap(); // the other client acts
	wait_until("first sees it", || mine(&first, "t2").first().is_some_and(|e| e.done)).await;
}

#[wasm_bindgen_test]
async fn a_new_connection_starts_with_the_current_model() {
	let first = Replica::new(Model::default());
	let link1 = connect(&first);
	wait_until("open", || link1.state() == State::Open).await;
	add(&link1, "t3 existing");
	wait_until("added", || mine(&first, "t3").len() == 1).await;

	let late = Replica::new(Model::default());
	let _link2 = connect(&late); // no action sent: the snapshot alone must bring it up to date
	wait_until("late client catches up", || mine(&late, "t3").len() == 1).await;
}

#[wasm_bindgen_test]
async fn bindings_only_run_when_their_field_changes() {
	let replica = Replica::new(Model::default());
	let link = connect(&replica);
	wait_until("open", || link.state() == State::Open).await;

	let counts = Rc::new(RefCell::new(Vec::new()));
	let sink = counts.clone();
	replica.bind(
		|model| model.items.iter().filter(|entry| entry.text.starts_with("t4")).count(),
		move |count| sink.borrow_mut().push(*count),
	);

	add(&link, "t4 a");
	wait_until("one item", || counts.borrow().last() == Some(&1)).await;
	add(&link, "other marker"); // changes the model, not this binding's field
	add(&link, "t4 b");
	wait_until("two items", || counts.borrow().last() == Some(&2)).await;
	assert_eq!(counts.borrow().iter().filter(|count| **count == 1).count(), 1);
}

#[wasm_bindgen_test]
async fn a_dropped_link_applies_nothing_more_and_cannot_send() {
	let (watching, acting) = (Replica::new(Model::default()), Replica::new(Model::default()));
	let (link1, link2) = (connect(&watching), connect(&acting));
	wait_until("both open", || link1.state() == State::Open && link2.state() == State::Open).await;
	let stale_actions = link1.actions();

	drop(link1);
	add(&link2, "t5 after drop");
	wait_until("the other client sees it", || mine(&acting, "t5").len() == 1).await;
	sleep(100).await;

	assert!(mine(&watching, "t5").is_empty());
	assert!(stale_actions.send(&Action::Add { text: "t5 late".into() }).is_err());
}

#[wasm_bindgen_test]
async fn a_frame_that_is_not_a_model_is_reported_and_leaves_the_replica_alone() {
	#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
	struct Wrong {
		nothing_like_the_server_sends: u32,
	}
	let replica = Replica::new(Wrong::default());
	let events = Rc::new(RefCell::new(Vec::new()));
	let sink = events.clone();
	let _link: Link<Wrong, Action> =
		Link::connect(URL, &replica, move |event| sink.borrow_mut().push(event)).unwrap();

	wait_until("a bad snapshot", || {
		events.borrow().iter().any(|event| matches!(event, LinkEvent::BadSnapshot(_)))
	})
	.await;
	assert!(events.borrow().contains(&LinkEvent::Open));
	assert_eq!(replica.get(), Wrong::default());
}

#[wasm_bindgen_test]
async fn the_server_ignores_an_ill_formed_action_and_keeps_the_same_connection_working() {
	let replica = Replica::new(Model::default());
	let link = connect(&replica);
	wait_until("open", || link.state() == State::Open).await;

	link.actions().send(&Action::DoesNotExist).unwrap(); // not a known action
	add(&link, "t6 still works"); // on the very same connection
	wait_until("the model still changes", || mine(&replica, "t6").len() == 1).await;
	assert_eq!(link.state(), State::Open);
}

#[wasm_bindgen_test]
async fn closing_the_connection_is_reported() {
	let replica = Replica::new(Model::default());
	let closed = Rc::new(Cell::new(false));
	let flag = closed.clone();
	let _link: Link<Model, Action> = Link::connect("ws://127.0.0.1:1/todo/ws", &replica, move |event| {
		if matches!(event, LinkEvent::Closed(_)) {
			flag.set(true);
		}
	})
	.unwrap();
	wait_until("a close event", || closed.get()).await;
}

// ---- a stream next to the model

#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
struct ViewerHub {
	viewers: u32,
	models: Vec<ModelInfo>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
struct ModelInfo {
	name: String,
	triangles: u32,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ViewerAction {
	Display { width: u32, height: u32 },
}

#[wasm_bindgen_test]
async fn binary_messages_reach_the_application_next_to_the_replicated_model() {
	// the viewer endpoint replicates a hub (text) and streams JPEG frames (binary) on one socket
	let replica = Replica::new(ViewerHub::default());
	let frames = Rc::new(RefCell::new(Vec::<Vec<u8>>::new()));
	let sink = frames.clone();
	let link: Link<ViewerHub, ViewerAction> =
		Link::connect("ws://127.0.0.1:2000/viewer/ws", &replica, move |event| {
			if let LinkEvent::Binary(bytes) = event {
				sink.borrow_mut().push(bytes);
			}
		})
		.unwrap();
	wait_until("the hub", || replica.get().viewers >= 1 && replica.get().models.len() == 4).await;
	assert!(replica.get().models.iter().all(|model| model.triangles > 0 && !model.name.is_empty()));

	// telling the server how big a frame is gets the shared scene rendered for this client; the hub's
	// `scene` field, which this model does not declare, is simply ignored
	link.actions().send(&ViewerAction::Display { width: 160, height: 120 }).unwrap();
	wait_until("two frames", || frames.borrow().len() >= 2).await;
	let frames = frames.borrow();
	for frame in frames.iter() {
		assert_eq!(&frame[..4], b"MOTH", "a viewer frame starts with its magic");
		assert_eq!(&frame[26..28], &[0xFF, 0xD8], "followed by a JPEG");
	}
	// the stream did not disturb the model: it is still what the server last said
	assert_eq!(replica.get().models.len(), 4);
}