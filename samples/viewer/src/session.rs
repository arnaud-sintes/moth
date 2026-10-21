//! The viewer's state and its conversation with the server.
//!
//! The scene (object, material, light, camera, turntable) is shared by everyone connected and lives
//! on the server: this page *asks* for changes (see [`Outbox`]), at most once per animation frame,
//! and shows what the server says the scene is. Only the size and quality of this browser's own
//! frames are its own. Frames come back as binary messages, are decoded and drawn, and each one is
//! acknowledged so that the server may send the next. Nothing here holds a component: the page's
//! callbacks hold an `Rc<Session>`, the session holds only weak handles to what it must poke.

use std::{
	cell::{Cell, OnceCell, RefCell},
	collections::VecDeque,
	rc::{Rc, Weak},
};

use js_sys::{Array, Uint8Array};
use moth::{Actions, CanvasHandle, Error, Size, Timeout, WeakProp, console};
use moth_sample_protocol::viewer::{ClientMsg, Display, FrameHeader, FrameKind, Material, Model, Patch, Scene};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, CanvasRenderingContext2d, ImageBitmap};

use crate::{orbit::Orbit, outbox::Outbox, stats::Stats};

/// How many sent views are remembered to time their round trip.
const SENT_MEMORY: usize = 128;
/// How often the HUD is refreshed.
const HUD_INTERVAL_MS: u32 = 400;

pub fn now_ms() -> f64 {
	web_sys::window().and_then(|window| window.performance()).map_or(0.0, |performance| performance.now())
}

/// The HUD's readouts, as handles: a text set after the page is gone does nothing.
pub struct Hud {
	pub fps: WeakProp<String>,
	pub latency: WeakProp<String>,
	pub frame: WeakProp<String>,
	pub rate: WeakProp<String>,
	pub size: WeakProp<String>,
	pub render: WeakProp<String>,
	pub encode: WeakProp<String>,
}

pub struct Session {
	// the camera as this page shows it (the server's, unless the user is moving it)
	orbit: Cell<Orbit>,
	drag: Cell<Option<(f64, f64)>>,
	/// What is still to be said about the scene.
	outbox: RefCell<Outbox>,

	// this browser's own: how big its frames are, and how finely they are rendered
	size: Cell<(u32, u32)>,
	scale: Cell<f32>,
	/// The size of the previews sent while the scene moves, as a fraction of the full frame's.
	preview: Cell<f32>,
	/// The JPEG quality of the full frame and of the previews.
	quality: Cell<u32>,
	preview_quality: Cell<u32>,
	display_dirty: Cell<bool>,

	// the conversation
	connected: Cell<bool>,
	/// When each patch was sent, to time the round trip of the one a preview shows.
	sent_at: RefCell<VecDeque<(u32, f64)>>,
	actions: OnceCell<Actions<ClientMsg>>,

	// frames
	/// The newest frame received and not yet decoded. A frame that is replaced before it was looked
	/// at is acknowledged on the spot: it still holds one of the server's credits.
	pending: RefCell<Option<(FrameHeader, Vec<u8>)>>,
	decoding: Cell<bool>,
	bitmap: RefCell<Option<ImageBitmap>>,

	stats: RefCell<Stats>,
	canvas: RefCell<Option<CanvasHandle>>,
	spark: RefCell<Option<CanvasHandle>>,
	hud: RefCell<Option<Hud>>,
	ticker: RefCell<Option<Timeout>>,
	me: Weak<Session>,
}

impl Session {
	pub fn new() -> Rc<Session> {
		Rc::new_cyclic(|me| Session {
			orbit: Cell::new(Orbit::HOME),
			drag: Cell::new(None),
			outbox: RefCell::new(Outbox::new((js_sys::Math::random() * f64::from(u32::MAX)) as u32)),
			size: Cell::new((0, 0)),
			scale: Cell::new(Display::default().scale),
			preview: Cell::new(Display::default().preview),
			quality: Cell::new(Display::default().quality),
			preview_quality: Cell::new(Display::default().preview_quality),
			display_dirty: Cell::new(true),
			connected: Cell::new(false),
			sent_at: RefCell::new(VecDeque::new()),
			actions: OnceCell::new(),
			pending: RefCell::new(None),
			decoding: Cell::new(false),
			bitmap: RefCell::new(None),
			stats: RefCell::new(Stats::default()),
			canvas: RefCell::new(None),
			spark: RefCell::new(None),
			hud: RefCell::new(None),
			ticker: RefCell::new(None),
			me: me.clone(),
		})
	}

	pub fn attach(&self, actions: Actions<ClientMsg>, canvas: CanvasHandle, spark: CanvasHandle, hud: Hud) {
		let _ = self.actions.set(actions);
		*self.canvas.borrow_mut() = Some(canvas);
		*self.spark.borrow_mut() = Some(spark);
		*self.hud.borrow_mut() = Some(hud);
	}

	// ---- what the user does

	/// Ask for a change to the shared scene. It is sent with the next animation frame, together with
	/// whatever else was asked for by then.
	pub fn request(&self, change: impl FnOnce(&mut Patch)) {
		self.outbox.borrow_mut().request(change);
		self.redraw();
	}

	pub fn set_model(&self, model: Model) {
		self.request(|patch| patch.model = Some(model));
	}

	pub fn set_material(&self, material: Material) {
		self.request(|patch| patch.material = Some(material));
	}

	pub fn set_light(&self, radians: f32) {
		self.request(|patch| patch.light = Some(radians));
	}

	pub fn set_shininess(&self, shininess: f32) {
		self.request(|patch| patch.shininess = Some(shininess));
	}

	pub fn set_wireframe(&self, on: bool) {
		self.request(|patch| patch.wireframe = Some(on));
	}

	pub fn set_spin(&self, on: bool) {
		self.request(|patch| patch.spin = Some(on));
	}

	/// This browser's own resolution setting (a fraction), not shared.
	pub fn set_scale(&self, scale: f32) {
		self.scale.set(scale);
		self.display_dirty.set(true);
		self.redraw();
	}

	/// How big the previews sent while the scene moves are, as a fraction of the full frame (this
	/// browser's own, not shared).
	pub fn set_preview(&self, preview: f32) {
		self.preview.set(preview);
		self.display_dirty.set(true);
		self.redraw();
	}

	/// The JPEG quality (10 to 100) of the full-quality frame sent when the scene rests (this
	/// browser's own).
	pub fn set_quality(&self, quality: u32) {
		self.quality.set(quality);
		self.display_dirty.set(true);
		self.redraw();
	}

	/// The JPEG quality of the previews sent while the scene moves (this browser's own).
	pub fn set_preview_quality(&self, quality: u32) {
		self.preview_quality.set(quality);
		self.display_dirty.set(true);
		self.redraw();
	}

	pub fn reset_camera(&self) {
		self.move_camera(|_| Orbit::HOME);
	}

	pub fn pointer_down(&self, x: f64, y: f64) {
		self.drag.set(Some((x, y)));
	}

	pub fn pointer_move(&self, x: f64, y: f64) {
		let Some((last_x, last_y)) = self.drag.get() else { return };
		self.drag.set(Some((x, y)));
		self.move_camera(|mut orbit| {
			orbit.drag((x - last_x) as f32, (y - last_y) as f32);
			orbit
		});
	}

	pub fn pointer_up(&self) {
		self.drag.set(None);
	}

	pub fn wheel(&self, delta: f64) {
		self.move_camera(|mut orbit| {
			orbit.zoom(delta as f32);
			orbit
		});
	}

	fn move_camera(&self, change: impl FnOnce(Orbit) -> Orbit) {
		self.orbit.set(change(self.orbit.get()));
		self.outbox.borrow_mut().camera_moved();
		self.redraw();
	}

	fn redraw(&self) {
		if let Some(canvas) = self.canvas.borrow().as_ref() {
			canvas.invalidate();
		}
	}

	// ---- the shared scene

	/// The server announced the scene. Returns whether it is to be shown (the caller then updates the
	/// controls): the echo of this page's own superseded edits is not, see [`Outbox::believe`].
	pub fn scene_announced(&self, scene: &Scene) -> bool {
		if !self.outbox.borrow().believe(scene) {
			return false;
		}
		self.orbit.set(Orbit { yaw: scene.yaw, pitch: scene.pitch, distance: scene.distance });
		true
	}

	// ---- the connection

	pub fn connection(&self, connected: bool) {
		self.connected.set(connected);
		if connected {
			// a new connection knows nothing of this browser: tell it the size of its frames, take the
			// scene it announces as the truth, and forget the old pacing
			self.display_dirty.set(true);
			self.outbox.borrow_mut().reconnected();
			self.sent_at.borrow_mut().clear();
		}
		self.redraw();
	}

	fn has_news_for_the_server(&self) -> bool {
		self.display_dirty.get() || self.outbox.borrow().is_waiting()
	}

	/// Say what changed. Called once per animation frame, so however fast the pointer moves the
	/// server is told at most 60 times a second.
	fn flush(&self) {
		if !self.connected.get() {
			return;
		}
		let (width, height) = self.size.get();
		if width == 0 || height == 0 {
			return;
		}
		let Some(actions) = self.actions.get() else { return };
		if self.display_dirty.get() {
			let display = Display {
				width,
				height,
				scale: self.scale.get(),
				preview: self.preview.get(),
				quality: self.quality.get(),
				preview_quality: self.preview_quality.get(),
			};
			if actions.send(&ClientMsg::Display(display)).is_ok() {
				self.display_dirty.set(false);
			}
		}
		let next = self.outbox.borrow().next(self.orbit.get());
		if let Some(patch) = next {
			if actions.send(&ClientMsg::Set(patch.clone())).is_ok() {
				self.outbox.borrow_mut().sent(&patch);
				let mut sent = self.sent_at.borrow_mut();
				if sent.len() == SENT_MEMORY {
					sent.pop_front();
				}
				sent.push_back((patch.seq, now_ms()));
			}
		}
	}

	fn acknowledge(&self, seq: u32) {
		if let Some(actions) = self.actions.get() {
			// not connected: the server that wanted this is gone, and the next one starts from zero
			let _ = actions.send(&ClientMsg::Ack { seq });
		}
	}

	// ---- frames

	/// A binary message from the server.
	pub fn frame(&self, message: Vec<u8>) {
		let (header, jpeg) = match FrameHeader::unpack(&message) {
			Ok((header, jpeg)) => (header, jpeg.to_vec()),
			Err(error) => {
				console::error(&format!("not a frame: {error}"));
				return;
			}
		};
		let superseded = self.pending.borrow_mut().replace((header, jpeg));
		if let Some((old, _)) = superseded {
			self.acknowledge(old.seq);
		}
		self.start_decoding();
	}

	fn start_decoding(&self) {
		if self.decoding.replace(true) {
			return;
		}
		let me = self.me.clone();
		spawn_local(async move {
			loop {
				// hold no strong reference across the await: the page may be gone by then
				let next = match me.upgrade() {
					Some(session) => session.pending.borrow_mut().take(),
					None => return,
				};
				let Some((header, jpeg)) = next else {
					if let Some(session) = me.upgrade() {
						session.decoding.set(false);
					}
					return;
				};
				let decoded = decode(&jpeg).await;
				let Some(session) = me.upgrade() else {
					if let Ok(bitmap) = decoded {
						bitmap.close();
					}
					return;
				};
				match decoded {
					Ok(bitmap) => session.show(&header, jpeg.len(), bitmap),
					Err(error) => {
						console::error(&format!("cannot decode a frame: {error}"));
						session.acknowledge(header.seq);
					}
				}
			}
		});
	}

	fn show(&self, header: &FrameHeader, bytes: usize, bitmap: ImageBitmap) {
		let now = now_ms();
		let previous = self.bitmap.borrow_mut().replace(bitmap);
		if let Some(previous) = previous {
			previous.close();
		}
		{
			let mut stats = self.stats.borrow_mut();
			stats.frame(now, bytes, header);
			// the round trip is the preview's: the final frame waits for the camera to rest on purpose
			if header.kind == FrameKind::Preview {
				// 0 names nobody's patch (somebody else's change, the turntable's); a patch is timed
				// once, by the first preview that shows it
				let mut sent_at = self.sent_at.borrow_mut();
				let position = sent_at.iter().position(|(id, _)| *id == header.view_id);
				if let (Some(position), true) = (position, header.view_id != 0) {
					if let Some((_, sent)) = sent_at.remove(position) {
						stats.latency(now - sent);
					}
				}
			}
		}
		self.acknowledge(header.seq);
		self.redraw();
	}

	// ---- drawing

	/// The canvas asks for a frame: keep the server told of the size, send what changed, and paint
	/// what has arrived.
	pub fn draw(&self, ctx: &CanvasRenderingContext2d, size: Size) {
		let measured = (size.width.round() as u32, size.height.round() as u32);
		if self.size.get() != measured {
			self.size.set(measured);
			self.display_dirty.set(true);
		}
		self.flush();

		ctx.set_fill_style_str("#05070d");
		ctx.fill_rect(0.0, 0.0, size.width, size.height);
		match self.bitmap.borrow().as_ref() {
			Some(bitmap) => {
				// the frame is as big as asked for times the resolution setting: stretch it to the canvas
				let _ = ctx.draw_image_with_image_bitmap_and_dw_and_dh(bitmap, 0.0, 0.0, size.width, size.height);
			}
			None => caption(ctx, size, "Waiting for the first frame\u{2026}"),
		}
		if !self.connected.get() {
			ctx.set_fill_style_str("rgba(5, 7, 13, 0.62)");
			ctx.fill_rect(0.0, 0.0, size.width, size.height);
			caption(ctx, size, "Connection lost, reconnecting\u{2026}");
		}
		// unsent changes need the next frame too (but not while there is nobody to send them to:
		// reconnecting redraws)
		if self.connected.get() && self.has_news_for_the_server() {
			self.redraw();
		}
	}

	/// The sparkline of recent round trips.
	pub fn draw_sparkline(&self, ctx: &CanvasRenderingContext2d, size: Size) {
		let samples: Vec<f64> = self.stats.borrow().history().collect();
		ctx.set_fill_style_str("rgba(255, 255, 255, 0.04)");
		ctx.fill_rect(0.0, 0.0, size.width, size.height);
		if samples.len() < 2 {
			return;
		}
		let top = samples.iter().copied().fold(40.0_f64, f64::max);
		let (step, bottom) = (size.width / (samples.len() - 1) as f64, size.height - 3.0);
		let y = |ms: f64| bottom - (ms / top) * (size.height - 8.0);
		ctx.begin_path();
		ctx.move_to(0.0, size.height);
		for (index, ms) in samples.iter().enumerate() {
			ctx.line_to(index as f64 * step, y(*ms));
		}
		ctx.line_to(size.width, size.height);
		ctx.close_path();
		ctx.set_fill_style_str("rgba(90, 212, 255, 0.16)");
		ctx.fill();
		ctx.begin_path();
		for (index, ms) in samples.iter().enumerate() {
			if index == 0 {
				ctx.move_to(0.0, y(*ms));
			} else {
				ctx.line_to(index as f64 * step, y(*ms));
			}
		}
		ctx.set_stroke_style_str("#5ad4ff");
		ctx.set_line_width(1.5);
		ctx.stroke();
	}

	// ---- the HUD

	/// Refresh the HUD now and again every [`HUD_INTERVAL_MS`], for as long as the session lives.
	pub fn start_ticker(&self) -> Result<(), Error> {
		self.refresh_hud();
		let me = self.me.clone();
		let next = Timeout::after(HUD_INTERVAL_MS, move || {
			if let Some(session) = me.upgrade() {
				let _ = session.start_ticker();
			}
		})?;
		// replacing the timer that is calling us is fine: a timeout may be dropped in its callback
		let previous = self.ticker.borrow_mut().replace(next);
		drop(previous);
		Ok(())
	}

	fn refresh_hud(&self) {
		let snapshot = self.stats.borrow_mut().snapshot(now_ms());
		if let Some(hud) = self.hud.borrow().as_ref() {
			hud.fps.set(format!("{:.0}", snapshot.fps));
			hud.rate.set(format!("{:.1} Mbit/s", snapshot.megabits));
			hud.latency.set(snapshot.latency_ms.map_or("\u{2013}".to_owned(), |ms| format!("{ms:.0} ms")));
			if let Some(last) = snapshot.last {
				let kind = if last.kind == FrameKind::Final { "refined" } else { "preview" };
				hud.frame.set(kind.to_owned());
				hud.size.set(format!("{}\u{d7}{}  {:.0} kB", last.width, last.height, last.bytes as f64 / 1000.0));
				hud.render.set(format!("{:.1} ms", last.render_ms));
				hud.encode.set(format!("{:.1} ms", last.encode_ms));
			}
		}
		if let Some(spark) = self.spark.borrow().as_ref() {
			spark.invalidate();
		}
	}
}

impl Drop for Session {
	fn drop(&mut self) {
		if let Some(bitmap) = self.bitmap.get_mut().take() {
			bitmap.close();
		}
	}
}

fn caption(ctx: &CanvasRenderingContext2d, size: Size, text: &str) {
	ctx.set_fill_style_str("#8c98b8");
	ctx.set_font("15px system-ui, sans-serif");
	ctx.set_text_align("center");
	let _ = ctx.fill_text(text, size.width / 2.0, size.height / 2.0);
}

/// JPEG bytes to a bitmap the canvas can draw, decoded by the browser off the main thread.
async fn decode(jpeg: &[u8]) -> Result<ImageBitmap, Error> {
	let bytes = Uint8Array::from(jpeg);
	let options = BlobPropertyBag::new();
	options.set_type("image/jpeg");
	let blob = Blob::new_with_u8_array_sequence_and_options(&Array::of1(&bytes), &options)?;
	let window = web_sys::window().ok_or_else(|| Error::new("no window"))?;
	let bitmap: JsValue = JsFuture::from(window.create_image_bitmap_with_blob(&blob)?).await?;
	bitmap.dyn_into::<ImageBitmap>().map_err(|_| Error::new("not an ImageBitmap"))
}
