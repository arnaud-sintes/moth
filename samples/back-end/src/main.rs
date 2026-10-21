//! Sample back-end, on a single port (the original used two):
//!
//! - `/ws` speaks the image-streaming sample protocol, `/` serves its front-end build;
//! - `/todo/ws` replicates the shared todo model (full snapshot to every client after each
//!   action), `/todo/` serves its front-end build;
//! - `/viewer/ws` renders a 3D object on request and streams JPEG frames (see `viewer.rs`),
//!   `/viewer/` serves its front-end build.
//!
//! ```text
//! moth-sample-back-end [--port 2000] [--bind 127.0.0.1] [--dir ../front-end/dist]
//!                      [--todo-dir ../todo/dist] [--viewer-dir ../viewer/dist]
//!                      [--allow-origin ORIGIN]... [--dev]
//! ```
//!
//! A browser lets any web page open a WebSocket to any address, so a page the user happens to have
//! open could drive this server (cross-site WebSocket hijacking). WebSocket upgrades are therefore
//! only accepted from the server's own origin (the page it serves), from the origins named with
//! `--allow-origin`, and, with `--dev`, from any page served by a loopback host (a `trunk serve` dev
//! server, the test runner). Requests with no `Origin` header (not from a browser) are accepted.

mod share;
mod viewer;

use std::{
	net::SocketAddr,
	path::PathBuf,
	sync::{Arc, Mutex, PoisonError},
};

use axum::{
	Router,
	extract::{
		ConnectInfo, State,
		ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
	},
	http::{HeaderMap, StatusCode, header},
	response::{IntoResponse, Response},
	routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use moth_sample_protocol::{
	BackToFront, FrontToBack,
	todo::{Action, Todos},
};
use tokio::{
	sync::{mpsc, watch},
	task::JoinHandle,
};
use tower_http::services::ServeDir;

const IMAGE: &[u8] = include_bytes!("../image.jpg");

/// Every message of either protocol is a small JSON object: anything bigger is refused by the
/// WebSocket layer (and closes the connection) before it is buffered.
const MAX_MESSAGE_BYTES: usize = 16 * 1024;

struct AppState {
	/// The streamed frame, serialized once: every `image` message is identical.
	frame: Utf8Bytes,
	todo: TodoHub,
	viewer: Arc<viewer::ViewerHub>,
	origins: OriginPolicy,
}

/// The one authoritative todo model, shared by every client.
struct TodoHub {
	model: Mutex<Todos>,
	/// The latest model as JSON. A watch channel keeps only the latest value, which is exactly
	/// what full-snapshot replication wants: a slow client skips intermediate states. The bytes are
	/// shared, so each client's copy is a reference count, not the whole model.
	snapshot: watch::Sender<Utf8Bytes>,
}

impl TodoHub {
	fn new() -> Self {
		let model = Todos::default();
		let (snapshot, _) = watch::channel(Self::json(&model));
		Self { model: Mutex::new(model), snapshot }
	}

	fn json(model: &Todos) -> Utf8Bytes {
		serde_json::to_string(model).expect("a model always serializes").into()
	}

	fn apply(&self, action: Action) {
		// the lock is held across the publish so snapshots are published in the order applied
		let mut model = self.model.lock().unwrap_or_else(PoisonError::into_inner);
		model.apply(action);
		self.snapshot.send_replace(Self::json(&model));
	}
}

/// Which pages may open a WebSocket here.
#[derive(Default)]
struct OriginPolicy {
	/// Accept any page served by a loopback host.
	dev: bool,
	/// Extra origins, exactly as a browser sends them (`http://127.0.0.1:8080`).
	allowed: Vec<String>,
}

impl OriginPolicy {
	/// `origin` and `host` are the request's `Origin` and `Host` headers.
	fn permits(&self, origin: Option<&str>, host: Option<&str>) -> bool {
		let Some(origin) = origin else { return true };
		if self.allowed.iter().any(|allowed| allowed.eq_ignore_ascii_case(origin)) {
			return true;
		}
		// "null" (a sandboxed frame, a file) and anything else without a scheme is never accepted
		let Some((_scheme, authority)) = origin.split_once("://") else { return false };
		if host.is_some_and(|host| host.eq_ignore_ascii_case(authority)) {
			return true; // same origin: the page this server served
		}
		self.dev && is_loopback(authority)
	}
}

fn is_loopback(authority: &str) -> bool {
	let host = match authority.strip_prefix('[') {
		Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
		None => authority.split(':').next().unwrap_or_default(),
	};
	matches!(host, "localhost" | "127.0.0.1" | "::1")
}

struct Settings {
	bind: String,
	port: u16,
	dir: PathBuf,
	todo_dir: PathBuf,
	viewer_dir: PathBuf,
	origins: OriginPolicy,
}

fn settings() -> Result<Settings, String> {
	let mut settings = Settings {
		bind: "127.0.0.1".into(),
		port: 2000,
		dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../front-end/dist").into(),
		todo_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../todo/dist").into(),
		viewer_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../viewer/dist").into(),
		origins: OriginPolicy::default(),
	};
	let mut args = std::env::args().skip(1);
	while let Some(arg) = args.next() {
		let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
		match arg.as_str() {
			"--port" => {
				settings.port = value("--port")?.parse().map_err(|e| format!("--port: {e}"))?;
			}
			"--bind" => settings.bind = value("--bind")?,
			"--dir" => settings.dir = value("--dir")?.into(),
			"--todo-dir" => settings.todo_dir = value("--todo-dir")?.into(),
			"--viewer-dir" => settings.viewer_dir = value("--viewer-dir")?.into(),
			"--allow-origin" => settings.origins.allowed.push(value("--allow-origin")?),
			"--dev" => settings.origins.dev = true,
			other => return Err(format!("unknown argument '{other}'")),
		}
	}
	Ok(settings)
}

#[tokio::main]
async fn main() {
	let settings = match settings() {
		Ok(settings) => settings,
		Err(message) => {
			eprintln!("{message}");
			std::process::exit(2);
		}
	};

	let frame = serde_json::to_string(&BackToFront {
		image: Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(IMAGE))),
		..Default::default()
	})
	.expect("a frame always serializes");
	let state = Arc::new(AppState {
		frame: frame.into(),
		todo: TodoHub::new(),
		viewer: viewer::ViewerHub::new(),
		origins: settings.origins,
	});

	for dir in [&settings.dir, &settings.todo_dir, &settings.viewer_dir] {
		if !dir.is_dir() {
			eprintln!(
				"warning: '{}' does not exist, its pages will 404 (build it with `trunk build`)",
				dir.display()
			);
		}
	}
	let app = Router::new()
		.route("/ws", get(upgrade))
		.route("/todo/ws", get(todo_upgrade))
		.route("/viewer/ws", get(viewer::upgrade))
		.nest_service("/todo", ServeDir::new(&settings.todo_dir))
		.nest_service("/viewer", ServeDir::new(&settings.viewer_dir))
		.fallback_service(ServeDir::new(&settings.dir))
		.with_state(state);

	let address = format!("{}:{}", settings.bind, settings.port);
	let listener = match tokio::net::TcpListener::bind(&address).await {
		Ok(listener) => listener,
		Err(error) => {
			eprintln!("cannot listen on {address}: {error}");
			std::process::exit(1);
		}
	};
	println!("> Http listening over folder '{}' on {address}", settings.dir.display());
	println!("> WebSocket listening on {address}/ws");
	println!("> Todo sample over folder '{}' on {address}/todo/", settings.todo_dir.display());
	println!("> 3D viewer over folder '{}' on {address}/viewer/", settings.viewer_dir.display());
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
		.await
		.expect("server failed");
}

/// `Some(refusal)` if the upgrade must not go ahead.
fn refuse_foreign_origin(state: &AppState, headers: &HeaderMap, peer: SocketAddr) -> Option<Response> {
	// a header that is present but not text is as suspect as a foreign one
	let read = |name| match headers.get(name) {
		None => Ok(None),
		Some(value) => value.to_str().map(Some).map_err(|_| ()),
	};
	let allowed = match (read(header::ORIGIN), read(header::HOST)) {
		(Ok(origin), Ok(host)) => state.origins.permits(origin, host),
		_ => false,
	};
	if allowed {
		return None;
	}
	// `Debug` on the header value escapes whatever a client put there
	eprintln!("[{peer}] WebSocket refused, origin {:?}", headers.get(header::ORIGIN));
	Some((StatusCode::FORBIDDEN, "origin not allowed").into_response())
}

async fn upgrade(
	socket: WebSocketUpgrade,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	State(state): State<Arc<AppState>>,
	headers: HeaderMap,
) -> Response {
	if let Some(refusal) = refuse_foreign_origin(&state, &headers, peer) {
		return refusal;
	}
	socket
		.max_message_size(MAX_MESSAGE_BYTES)
		.max_frame_size(MAX_MESSAGE_BYTES)
		.on_upgrade(move |socket| connection(socket, peer, state))
}

/// What the client sent, as text; `None` for frames that carry none (ping, pong) and an `Err` when
/// the connection is over (`clean`: the client said goodbye; otherwise it was lost).
enum Incoming {
	Text(String),
	Ignore,
	Over { clean: bool },
}

fn classify(message: Option<Result<Message, axum::Error>>) -> Incoming {
	match message {
		Some(Ok(Message::Text(text))) => Incoming::Text(text.as_str().to_owned()),
		// the first protocol also accepted JSON in binary frames
		Some(Ok(Message::Binary(bytes))) => match std::str::from_utf8(&bytes) {
			Ok(text) => Incoming::Text(text.to_owned()),
			Err(_) => Incoming::Ignore,
		},
		Some(Ok(Message::Close(_))) => Incoming::Over { clean: true },
		Some(Ok(_)) => Incoming::Ignore,
		Some(Err(_)) | None => Incoming::Over { clean: false },
	}
}

fn how(clean: bool) -> &'static str {
	if clean { "clean" } else { "connection lost" }
}

/// One client. Everything it owns (writer task, streaming task) ends with it.
async fn connection(socket: WebSocket, peer: SocketAddr, state: Arc<AppState>) {
	println!("[{peer}] WebSocket client connection");
	let (mut sink, mut incoming) = socket.split();

	// a single writer serializes every send; the small bound makes streaming respect backpressure
	let (outgoing, mut queue) = mpsc::channel::<Message>(4);
	let writer = tokio::spawn(async move {
		while let Some(message) = queue.recv().await {
			if sink.send(message).await.is_err() {
				break;
			}
		}
		// completes the closing handshake: after a client `Close` this flushes our reply frame,
		// without it the client sees an abnormal 1006 closure
		let _ = sink.close().await;
	});

	let greeting = BackToFront {
		message: Some(format!("(Back-End) you're now connected '{peer}'")),
		..Default::default()
	};
	let _ = outgoing
		.send(Message::Text(
			serde_json::to_string(&greeting).expect("a greeting always serializes").into(),
		))
		.await;

	// the stream belongs to this connection (the original kept one global timer for all clients)
	let mut streaming: Option<JoinHandle<()>> = None;
	let clean = loop {
		let text = match classify(incoming.next().await) {
			Incoming::Text(text) => text,
			Incoming::Ignore => continue,
			Incoming::Over { clean } => break clean,
		};
		// what a client sends is data: it is logged escaped (`Debug`), never raw
		match serde_json::from_str::<FrontToBack>(&text) {
			Ok(FrontToBack { message: Some(message), .. }) => {
				println!("[{peer}] WebSocket receive {message:?}");
			}
			Ok(FrontToBack { stream: Some(active), .. }) => {
				println!("[{peer}] Image streaming: {}", if active { "active" } else { "inactive" });
				if let Some(task) = streaming.take() {
					task.abort();
				}
				if active {
					streaming = Some(tokio::spawn(stream(outgoing.clone(), state.frame.clone())));
				}
			}
			Ok(_) => {}
			Err(error) => eprintln!("[{peer}] ill-formed message ignored: {error}"),
		}
	};

	if let Some(task) = streaming.take() {
		task.abort();
	}
	drop(outgoing);
	let _ = writer.await;
	println!("[{peer}] WebSocket client disconnection ({})", how(clean));
}

async fn stream(outgoing: mpsc::Sender<Message>, frame: Utf8Bytes) {
	while outgoing.send(Message::Text(frame.clone())).await.is_ok() {}
}

async fn todo_upgrade(
	socket: WebSocketUpgrade,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	State(state): State<Arc<AppState>>,
	headers: HeaderMap,
) -> Response {
	if let Some(refusal) = refuse_foreign_origin(&state, &headers, peer) {
		return refusal;
	}
	socket
		.max_message_size(MAX_MESSAGE_BYTES)
		.max_frame_size(MAX_MESSAGE_BYTES)
		.on_upgrade(move |socket| todo_connection(socket, peer, state))
}

/// One todo client: it receives the current model on connect and after every change made by
/// anyone, and its text frames are actions for the reducer.
async fn todo_connection(socket: WebSocket, peer: SocketAddr, state: Arc<AppState>) {
	println!("[{peer}] todo client connected");
	let (mut sink, mut incoming) = socket.split();
	let mut snapshots = state.todo.snapshot.subscribe();

	let mut clean = false;
	let current = snapshots.borrow_and_update().clone();
	if sink.send(Message::Text(current)).await.is_ok() {
		loop {
			tokio::select! {
				changed = snapshots.changed() => {
					if changed.is_err() {
						break;
					}
					let json = snapshots.borrow_and_update().clone();
					if sink.send(Message::Text(json)).await.is_err() {
						break;
					}
				}
				message = incoming.next() => match classify(message) {
					Incoming::Text(text) => match serde_json::from_str::<Action>(&text) {
						Ok(action) => state.todo.apply(action),
						Err(error) => eprintln!("[{peer}] ill-formed action ignored: {error}"),
					},
					Incoming::Ignore => {}
					Incoming::Over { clean: ended_cleanly } => {
						clean = ended_cleanly;
						break;
					}
				},
			}
		}
	}
	// completes the closing handshake, see `connection`
	let _ = sink.close().await;
	println!("[{peer}] todo client disconnected ({})", how(clean));
}

#[cfg(test)]
mod tests {
	use super::*;

	fn policy(dev: bool, allowed: &[&str]) -> OriginPolicy {
		OriginPolicy { dev, allowed: allowed.iter().map(|origin| (*origin).to_owned()).collect() }
	}

	const HOST: Option<&str> = Some("127.0.0.1:2000");

	#[test]
	fn a_request_without_an_origin_is_not_from_a_browser_and_is_accepted() {
		assert!(policy(false, &[]).permits(None, HOST));
	}

	#[test]
	fn the_page_this_server_served_is_accepted() {
		assert!(policy(false, &[]).permits(Some("http://127.0.0.1:2000"), HOST));
		assert!(policy(false, &[]).permits(Some("https://127.0.0.1:2000"), HOST));
		assert!(policy(false, &[]).permits(Some("HTTP://127.0.0.1:2000"), HOST));
	}

	#[test]
	fn any_other_page_is_refused_by_default() {
		let policy = policy(false, &[]);
		assert!(!policy.permits(Some("https://evil.example"), HOST));
		assert!(!policy.permits(Some("http://127.0.0.1:8080"), HOST)); // same host, other port
		assert!(!policy.permits(Some("http://localhost:2000"), HOST)); // same port, other name
		assert!(!policy.permits(Some("http://evil.example:2000"), HOST));
	}

	#[test]
	fn an_origin_without_a_scheme_is_accepted_only_if_it_is_named() {
		// even in dev mode: a sandboxed frame or a file page sends "null"
		let dev = policy(true, &[]);
		for origin in ["null", "127.0.0.1:2000", ""] {
			assert!(!dev.permits(Some(origin), HOST), "{origin:?}");
		}
		assert!(policy(false, &["null"]).permits(Some("null"), HOST));
	}

	#[test]
	fn named_origins_are_accepted_exactly() {
		let policy = policy(false, &["http://127.0.0.1:8080"]);
		assert!(policy.permits(Some("http://127.0.0.1:8080"), HOST));
		assert!(policy.permits(Some("HTTP://127.0.0.1:8080"), HOST));
		assert!(!policy.permits(Some("http://127.0.0.1:8081"), HOST));
		assert!(!policy.permits(Some("http://127.0.0.1:8080.evil.example"), HOST));
	}

	#[test]
	fn dev_mode_accepts_loopback_pages_only() {
		let policy = policy(true, &[]);
		assert!(policy.permits(Some("http://127.0.0.1:8080"), HOST));
		assert!(policy.permits(Some("http://localhost:3000"), HOST));
		assert!(policy.permits(Some("http://[::1]:4000"), HOST));
		assert!(!policy.permits(Some("https://evil.example"), HOST));
		assert!(!policy.permits(Some("http://127.0.0.1.evil.example:80"), HOST));
		assert!(!policy.permits(Some("http://localhost.evil.example"), HOST));
	}

	#[test]
	fn a_missing_host_header_only_loses_the_same_origin_rule() {
		assert!(!policy(false, &[]).permits(Some("http://127.0.0.1:2000"), None));
		assert!(policy(true, &[]).permits(Some("http://127.0.0.1:2000"), None));
	}
}
