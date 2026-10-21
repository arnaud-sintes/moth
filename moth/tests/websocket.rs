//! WebSocket lifetime and error behaviour.
//!
//! Needs the sample back-end listening on 127.0.0.1:2000 (see scripts/test-wasm.ps1, which starts
//! it). Its protocol: a greeting `{"message":...}` on connect, `{"stream":true|false}` toggles a
//! flood of `{"image":...}` frames.

use std::{cell::{Cell, RefCell}, rc::Rc};

mod common;

use common::{ErrorWatch, sleep, wait_until};
use moth::{Close, Handlers, Message, State, WebSocket};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

const SERVER: &str = "ws://127.0.0.1:2000/ws";

fn is_text(message: &Message, needle: &str) -> bool {
	matches!(message, Message::Text(text) if text.contains(needle))
}

#[wasm_bindgen_test]
async fn connects_and_receives_the_greeting() {
	let received = Rc::new(RefCell::new(Vec::new()));
	let opened = Rc::new(Cell::new(false));
	let (sink, flag) = (received.clone(), opened.clone());
	let socket = WebSocket::connect(
		SERVER,
		Handlers::new()
			.on_open(move |socket| {
				assert_eq!(socket.state(), State::Open);
				flag.set(true);
			})
			.on_message(move |_, message| sink.borrow_mut().push(message)),
	)
	.unwrap();

	wait_until("greeting", || !received.borrow().is_empty()).await;
	assert!(opened.get());
	assert!(is_text(&received.borrow()[0], "you're now connected"));
	assert_eq!(socket.state(), State::Open);
}

#[wasm_bindgen_test]
async fn sending_before_the_socket_is_open_is_an_error_not_a_silent_loss() {
	let socket = WebSocket::connect(SERVER, Handlers::new()).unwrap();
	let error = socket.send_text("too early").unwrap_err();
	assert!(error.to_string().contains("Connecting"), "{error}");
}

#[wasm_bindgen_test]
async fn callbacks_can_answer_through_the_handle_they_receive() {
	let frames = Rc::new(Cell::new(0));
	let counter = frames.clone();
	let _socket = WebSocket::connect(
		SERVER,
		Handlers::new()
			.on_open(|socket| socket.send_text(r#"{"stream":true}"#).unwrap())
			.on_message(move |socket, message| {
				if is_text(&message, "\"image\"") {
					counter.set(counter.get() + 1);
					if counter.get() == 3 {
						socket.send_text(r#"{"stream":false}"#).unwrap();
					}
				}
			}),
	)
	.unwrap();
	wait_until("three frames", || frames.get() >= 3).await;
}

#[wasm_bindgen_test]
async fn no_callback_runs_after_the_socket_is_dropped() {
	// audit: "callbacks capture raw `this`, destructor does not guarantee no in-flight callback"
	//
	// `message` events cannot prove it: a socket that is closing no longer delivers them, listeners
	// or not. The `close` event, which the browser fires once the closing handshake is over, can:
	// it reaches `on_close` only if the listener were still attached.
	let errors = ErrorWatch::new();
	let (frames, closes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
	let (counter, closed) = (frames.clone(), closes.clone());
	let socket = WebSocket::connect(
		SERVER,
		Handlers::new()
			.on_open(|socket| socket.send_text(r#"{"stream":true}"#).unwrap())
			.on_message(move |_, _| counter.set(counter.get() + 1))
			.on_close(move |_, _| closed.set(closed.get() + 1)),
	)
	.unwrap();
	let handle = socket.handle();

	wait_until("frames flowing", || frames.get() >= 5).await;
	drop(socket); // the server is still flooding: frames are in flight
	let at_drop = frames.get();
	wait_until("the browser to finish closing", || handle.state() == State::Closed).await;
	sleep(100).await;

	assert_eq!(frames.get(), at_drop);
	assert_eq!(closes.get(), 0, "on_close ran after the socket was dropped");
	errors.assert_none();
	let error = handle.send_text("late").unwrap_err();
	assert!(error.to_string().contains("cannot send"), "{error}");
}

#[wasm_bindgen_test]
async fn a_callback_can_drop_the_socket_that_owns_it() {
	let slot: Rc<RefCell<Option<WebSocket>>> = Rc::default();
	let handler_slot = slot.clone();
	let calls = Rc::new(Cell::new(0));
	let counter = calls.clone();
	let socket = WebSocket::connect(
		SERVER,
		Handlers::new().on_message(move |_, _| {
			counter.set(counter.get() + 1);
			let socket = handler_slot.borrow_mut().take();
			drop(socket);
		}),
	)
	.unwrap();
	let handle = socket.handle();
	*slot.borrow_mut() = Some(socket);

	wait_until("greeting handled", || calls.get() == 1).await;
	assert!(slot.borrow().is_none());
	wait_until("socket closed", || handle.state() == State::Closed).await;
	assert_eq!(calls.get(), 1);
}

#[wasm_bindgen_test]
async fn a_clean_close_reports_through_on_close() {
	let closed: Rc<RefCell<Option<Close>>> = Rc::default();
	let sink = closed.clone();
	let socket = WebSocket::connect(
		SERVER,
		Handlers::new()
			.on_open(|socket| socket.send_text(r#"{"message":"closing"}"#).unwrap())
			.on_close(move |_, close| *sink.borrow_mut() = Some(close)),
	)
	.unwrap();
	wait_until("open", || socket.state() == State::Open).await;
	socket.close().unwrap();

	wait_until("close event", || closed.borrow().is_some()).await;
	let close = closed.borrow().clone().unwrap();
	assert_eq!(close.code, 1000);
	assert!(close.was_clean);
	assert_eq!(socket.state(), State::Closed);
}

#[wasm_bindgen_test]
async fn an_unreachable_server_reports_error_then_unclean_close() {
	let events = Rc::new(RefCell::new(Vec::<String>::new()));
	let (on_error, on_close) = (events.clone(), events.clone());
	let _socket = WebSocket::connect(
		"ws://127.0.0.1:1/ws",
		Handlers::new()
			.on_error(move |_| on_error.borrow_mut().push("error".into()))
			.on_close(move |_, close| {
				on_close.borrow_mut().push(format!("close clean={}", close.was_clean))
			}),
	)
	.unwrap();
	wait_until("close", || events.borrow().iter().any(|e| e.starts_with("close"))).await;
	assert_eq!(*events.borrow(), ["error", "close clean=false"]);
}

#[wasm_bindgen_test]
fn a_malformed_url_is_an_error_not_a_panic() {
	// not "not a url": browsers resolve relative URLs against the page, so that one is valid
	let error = WebSocket::connect("ftp://example.com/", Handlers::new()).err().unwrap();
	assert!(error.to_string().contains("SyntaxError"), "{error}");
}
