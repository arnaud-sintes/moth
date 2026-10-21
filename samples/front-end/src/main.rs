use std::{cell::OnceCell, rc::Rc};

use moth::{
	Body, Component, Error, Handlers, Image, Label, Message, PushButton, SocketHandle, ToggleButton, WebSocket,
	console,
};
use moth_sample_protocol::{BackToFront, FrontToBack};

/// The page owns its whole component tree and the socket: dropping it removes everything from the
/// DOM and closes the connection, and no callback can run afterwards.
struct Page {
	_body: Body,
	_title: Label,
	_send: PushButton,
	_state: Label,
	_stream: ToggleButton,
	_fps: Label,
	_image: Image,
	_socket: WebSocket,
}

impl Page {
	fn new() -> Result<Self, Error> {
		// the elements already written in index.html become components
		moth::define_all()?;

		let body = Body::new()?;
		let title = Label::new(&body, "> moth demonstration")?;

		// the buttons are created before the socket, so they reach it through a slot filled once it
		// exists; until then a click finds nothing and does nothing
		let socket_slot: Rc<OnceCell<SocketHandle>> = Rc::default();

		let slot = socket_slot.clone();
		let send_button = PushButton::new(&body, "WebSocket send", move || {
			if let Some(socket) = slot.get() {
				send(socket, &message("(Front-End) clicking send button"));
			}
		})?;

		let state = Label::new(&body, &stream_text(false))?;
		let state_text = state.text_handle();
		let slot = socket_slot.clone();
		let stream = ToggleButton::new(&body, "Stream", move |active| {
			state_text.set(stream_text(active));
			if let Some(socket) = slot.get() {
				send(socket, &stream_order(active));
			}
		})?;

		let fps = Label::new(&body, "fps: -")?;
		let image = Image::new(&body)?;

		// stable ids, for styling and for the end-to-end script
		send_button.element().set_id("send-button");
		state.element().set_id("stream-state");
		stream.element().set_id("streaming-toggle");
		fps.element().set_id("stream-fps");
		image.element().set_id("stream-image");

		let fps_text = fps.text_handle();
		let image_source = image.source_handle();
		let toggled = stream.toggled_handle();
		let mut frames = FrameCounter::new();
		let socket = WebSocket::connect(
			&back_end_url()?,
			Handlers::new()
				.on_open(move |socket| {
					send(socket, &message("(Front-End) WebSocket is opened and ready"));
					// the toggle may have been flipped before the connection was ready
					if let Some(active) = toggled.get() {
						send(socket, &stream_order(active));
					}
				})
				.on_message(move |_, received| {
					let Message::Text(text) = received else { return };
					match serde_json::from_str::<BackToFront>(&text) {
						Ok(BackToFront { message: Some(message), .. }) => {
							console::log(&format!("[Server] WebSocket receive '{message}'"));
						}
						Ok(BackToFront { image: Some(image), .. }) => {
							if let Some(fps) = frames.tick() {
								fps_text.set(format!("fps: {fps:.1}"));
							}
							image_source.set(image);
						}
						Ok(_) => {}
						Err(error) => console::error(&format!("badly formatted data: {error}")),
					}
				})
				.on_close(|_, close| {
					console::log(&format!("[Server] WebSocket terminated ({})", close.code));
				}),
		)?;
		let _ = socket_slot.set(socket.handle());

		Ok(Self {
			_body: body,
			_title: title,
			_send: send_button,
			_state: state,
			_stream: stream,
			_fps: fps,
			_image: image,
			_socket: socket,
		})
	}
}

/// The back-end's WebSocket URL: the same origin as the page (the back-end serves the page, and
/// allows only its own pages by default), over `wss` if the page is `https`. For a page that comes
/// from elsewhere (`trunk serve`), `?backend=127.0.0.1:2000` names the back-end.
fn back_end_url() -> Result<String, Error> {
	let location = web_sys::window().ok_or_else(|| Error::new("no window"))?.location();
	let scheme = if location.protocol()? == "https:" { "wss" } else { "ws" };
	let backend = web_sys::UrlSearchParams::new_with_str(&location.search()?)?.get("backend");
	let host = match backend {
		Some(backend) => backend,
		None => location.host()?,
	};
	Ok(format!("{scheme}://{host}/ws"))
}

fn message(text: &str) -> FrontToBack {
	FrontToBack { message: Some(text.into()), ..Default::default() }
}

fn stream_order(active: bool) -> FrontToBack {
	FrontToBack { stream: Some(active), ..Default::default() }
}

fn stream_text(active: bool) -> String {
	format!("Image streaming: {}", if active { "active" } else { "inactive" })
}

/// A failed send is reported, never silent or fatal.
fn send(socket: &SocketHandle, data: &FrontToBack) {
	let result = serde_json::to_string(data)
		.map_err(|error| error.to_string())
		.and_then(|json| socket.send_text(&json).map_err(|error| error.to_string()));
	if let Err(error) = result {
		console::error(&format!("fail to send: {error}"));
	}
}

/// Frames per second, measured over windows of at least one second.
struct FrameCounter {
	frames: u32,
	since: f64,
}

impl FrameCounter {
	fn new() -> Self {
		Self { frames: 0, since: js_sys::Date::now() }
	}

	/// Count a frame; returns the rate when a window just completed.
	fn tick(&mut self) -> Option<f64> {
		self.frames += 1;
		let now = js_sys::Date::now();
		let elapsed = now - self.since;
		if elapsed < 1000.0 {
			return None;
		}
		let fps = f64::from(self.frames) * 1000.0 / elapsed;
		self.frames = 0;
		self.since = now;
		Some(fps)
	}
}

fn main() {
	moth::init();
	let page = Page::new().expect("page creation failed");
	moth::mount(page).expect("mount failed");
}
