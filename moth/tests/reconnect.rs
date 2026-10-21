//! Link reconnection. Needs the sample back-end on 127.0.0.1:2000 (see scripts/test-wasm.ps1) for the
//! tests that really connect; the others use a port nothing listens on. That the link recovers from a
//! server that really goes away and comes back is checked in scripts/e2e-todo.ps1, which can restart it.

mod common;

use std::{cell::RefCell, rc::Rc};

use common::{ErrorWatch, sleep, wait_until};
use moth::{Backoff, Link, LinkEvent, LinkOptions, Replica, State};
use serde::{Deserialize, Serialize};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{PageTransitionEvent, PageTransitionEventInit};

wasm_bindgen_test_configure!(run_in_browser);

const URL: &str = "ws://127.0.0.1:2000/todo/ws";
const NOBODY: &str = "ws://127.0.0.1:1/todo/ws";

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
}

type Events = Rc<RefCell<Vec<LinkEvent>>>;

fn reconnecting(backoff: Backoff) -> LinkOptions {
	LinkOptions { reconnect: Some(backoff) }
}

fn connect(url: &str, options: LinkOptions, replica: &Replica<Model>, events: &Events) -> Link<Model, Action> {
	let sink = events.clone();
	Link::connect_with(url, replica, options, move |event| sink.borrow_mut().push(event)).unwrap()
}

fn opens(events: &Events) -> usize {
	events.borrow().iter().filter(|event| **event == LinkEvent::Open).count()
}

fn mine(replica: &Replica<Model>, marker: &str) -> Vec<Entry> {
	replica.get().items.into_iter().filter(|entry| entry.text.starts_with(marker)).collect()
}

fn page_show(persisted: bool) {
	let init = PageTransitionEventInit::new();
	init.set_persisted(persisted);
	let event = PageTransitionEvent::new_with_event_init_dict("pageshow", &init).unwrap();
	web_sys::window().unwrap().dispatch_event(&event).unwrap();
}

// ---- the policy

#[wasm_bindgen_test]
fn the_backoff_doubles_up_to_its_maximum() {
	let backoff = Backoff { initial_ms: 100, max_ms: 1000 };
	let waits: Vec<u32> = (0..6).map(|attempt| backoff.delay_with(attempt, 0.5)).collect();
	assert_eq!(waits, [100, 200, 400, 800, 1000, 1000]);
	assert_eq!(backoff.delay_with(1000, 0.5), 1000); // no overflow for a long outage
}

#[wasm_bindgen_test]
fn the_backoff_is_randomized_within_twenty_percent_and_never_exceeds_its_maximum() {
	let backoff = Backoff { initial_ms: 100, max_ms: 1000 };
	assert_eq!(backoff.delay_with(1, 0.0), 160); // 200 * 0.8
	assert_eq!(backoff.delay_with(1, 1.0), 240); // 200 * 1.2
	assert_eq!(backoff.delay_with(4, 1.0), 1000); // 1000 * 1.2 is capped
	for _ in 0..200 {
		let wait = backoff.delay_ms(2); // real randomness
		assert!((320..=480).contains(&wait), "{wait}");
	}
}

#[wasm_bindgen_test]
fn the_backoff_never_waits_zero() {
	assert_eq!(Backoff { initial_ms: 0, max_ms: 0 }.delay_with(0, 0.5), 1);
	assert_eq!(Backoff { initial_ms: 1, max_ms: 1 }.delay_with(0, 0.0), 1);
}

// ---- retrying

#[wasm_bindgen_test]
async fn without_reconnection_a_lost_connection_stays_lost() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(NOBODY, LinkOptions::default(), &replica, &events);
	wait_until("the close", || events.borrow().iter().any(|e| matches!(e, LinkEvent::Closed(_)))).await;
	sleep(300).await;
	assert!(events.borrow().iter().all(|e| matches!(e, LinkEvent::Closed(_))), "{:?}", events.borrow());
	assert_eq!(link.state(), State::Closed);
}

#[wasm_bindgen_test]
async fn an_unreachable_server_is_retried_with_a_growing_wait() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let _link = connect(NOBODY, reconnecting(Backoff { initial_ms: 20, max_ms: 80 }), &replica, &events);
	let attempts = || events.borrow().iter().filter(|e| matches!(e, LinkEvent::Reconnecting { .. })).count();
	wait_until("four attempts", || attempts() >= 4).await;

	let waits: Vec<(u32, u32)> = events
		.borrow()
		.iter()
		.filter_map(|event| match event {
			LinkEvent::Reconnecting { attempt, delay_ms } => Some((*attempt, *delay_ms)),
			_ => None,
		})
		.take(4)
		.collect();
	assert_eq!(waits.iter().map(|(attempt, _)| *attempt).collect::<Vec<_>>(), [1, 2, 3, 4]);
	let within = |wait: u32, base: f64| (base * 0.8..=base * 1.2).contains(&f64::from(wait));
	assert!(within(waits[0].1, 20.0) && within(waits[1].1, 40.0), "{waits:?}");
	assert!(waits[2].1 >= 64 && waits[2].1 <= 80 && waits[3].1 >= 64 && waits[3].1 <= 80, "{waits:?}");

	// every wait follows a close
	let kinds: Vec<&str> = events
		.borrow()
		.iter()
		.take(6)
		.map(|event| match event {
			LinkEvent::Closed(_) => "closed",
			LinkEvent::Reconnecting { .. } => "reconnecting",
			_ => "other",
		})
		.collect();
	assert_eq!(kinds, ["closed", "reconnecting", "closed", "reconnecting", "closed", "reconnecting"]);
}

#[wasm_bindgen_test]
async fn dropping_the_link_cancels_the_pending_reconnection() {
	let errors = ErrorWatch::new();
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(NOBODY, reconnecting(Backoff { initial_ms: 150, max_ms: 150 }), &replica, &events);
	wait_until("a wait", || events.borrow().iter().any(|e| matches!(e, LinkEvent::Reconnecting { .. }))).await;

	drop(link); // the attempt is scheduled but has not happened
	let at_drop = events.borrow().len();
	sleep(400).await; // well past when it would have fired, twice over
	assert_eq!(events.borrow().len(), at_drop);
	// a timer left running would call into the dropped closure, which is only reported
	errors.assert_none();
}

// ---- really reconnecting

#[wasm_bindgen_test]
async fn a_reconnection_repairs_a_stale_replica_and_earlier_actions_keep_working() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(URL, reconnecting(Backoff::default()), &replica, &events);
	let actions = link.actions(); // taken before the reconnection
	wait_until("open", || opens(&events) == 1).await;
	actions.send(&Action::Add { text: "rc1 first".into() }).unwrap();
	wait_until("the item", || mine(&replica, "rc1").len() == 1).await;

	replica.apply(Model::default()); // the page's copy goes stale
	assert!(mine(&replica, "rc1").is_empty());
	link.reconnect().unwrap();
	wait_until("a second open", || opens(&events) == 2).await;
	wait_until("the snapshot repairs the copy", || mine(&replica, "rc1").len() == 1).await;

	actions.send(&Action::Add { text: "rc1 second".into() }).unwrap();
	wait_until("an action sent through the old handle", || mine(&replica, "rc1").len() == 2).await;
}

#[wasm_bindgen_test]
async fn reconnecting_by_hand_works_without_automatic_reconnection() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(URL, LinkOptions::default(), &replica, &events);
	wait_until("open", || opens(&events) == 1).await;
	link.reconnect().unwrap();
	wait_until("open again", || opens(&events) == 2).await;
	// the abandoned connection reported nothing
	assert!(!events.borrow().iter().any(|event| matches!(event, LinkEvent::Closed(_))));
}

#[wasm_bindgen_test]
async fn a_page_restored_from_the_back_forward_cache_reconnects() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(URL, reconnecting(Backoff::default()), &replica, &events);
	wait_until("open", || opens(&events) == 1).await;

	page_show(false); // an ordinary page show: nothing to repair
	sleep(150).await;
	assert_eq!(opens(&events), 1);

	page_show(true); // restored from the cache
	wait_until("a new connection", || opens(&events) == 2).await;
	drop(link);
	page_show(true); // the link is gone: nothing may react
	sleep(150).await;
	assert_eq!(opens(&events), 2);
}

#[wasm_bindgen_test]
async fn sending_while_there_is_no_connection_is_an_error_and_a_dropped_link_cannot_send() {
	let events: Events = Rc::default();
	let replica = Replica::new(Model::default());
	let link = connect(NOBODY, reconnecting(Backoff { initial_ms: 500, max_ms: 500 }), &replica, &events);
	let actions = link.actions();
	wait_until("a close", || events.borrow().iter().any(|e| matches!(e, LinkEvent::Closed(_)))).await;
	assert!(actions.send(&Action::Add { text: "nobody hears".into() }).is_err());

	drop(link);
	let error = actions.send(&Action::Add { text: "nor now".into() }).unwrap_err();
	assert!(error.to_string().contains("dropped"), "{error}");
}
