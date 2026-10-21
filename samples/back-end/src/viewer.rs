//! The remote viewer: everyone connected shares one [`Scene`] (the object, its material, the light,
//! the camera, the turntable), and each browser is streamed JPEG frames of it at its own size.
//!
//! Shared, in the [`ViewerHub`]: the scene lives here, a patch from any connection changes it, and
//! every connection is told the new state (replicated like the todo model). While the turntable is
//! on the server turns the camera for everyone. One task renders for all the viewers:
//!
//! - it is given the newest picture of the scene whenever it has something to do, so a renderer slower
//!   than the changes never builds a queue: it always renders the newest state (**latest wins**);
//! - a viewer is sent at most [`FRAMES_IN_FLIGHT`] frames it has not acknowledged (**credits**), which
//!   paces the stream to what each browser can draw and keeps one that stops reading from making the
//!   server render for nobody, without ever holding up the others;
//! - while the picture keeps changing, small fast previews are sent; once it has rested for [`SETTLE`],
//!   one full-size anti-aliased frame follows (**progressive refinement**);
//! - viewers whose screens are alike are served by **one rendering**, reduced on the server before it
//!   is compressed for those that are a bit smaller, see `share.rs`; only a very different screen is
//!   rendered for separately;
//! - what the client sends is untrusted: patches and displays are bounded before they are used.

use std::{
	net::SocketAddr,
	sync::{
		Arc, LazyLock, Mutex, MutexGuard, PoisonError,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

use axum::{
	extract::{
		ConnectInfo, State,
		ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
	},
	http::HeaderMap,
	response::Response,
};
use futures_util::{SinkExt, StreamExt};
use moth_sample_protocol::viewer::{
	ClientMsg, Display, FRAMES_IN_FLIGHT, FrameHeader, FrameKind, Hub, Material, Model, ModelInfo, Patch, Scene,
};
use moth_sample_render::{Image, Material as RenderMaterial, Mesh, Settings};
use tokio::{
	sync::{Notify, mpsc, watch},
	time::{MissedTickBehavior, interval, timeout_at},
};

use crate::{
	AppState, Incoming, MAX_MESSAGE_BYTES, classify, how, refuse_foreign_origin,
	share::{self, Rendering, Want, want},
};

/// How long the picture must rest before the full-quality frame is rendered.
const SETTLE: Duration = Duration::from_millis(220);
/// How often the turntable moves the camera for everyone.
const TURN_EVERY: Duration = Duration::from_millis(16);

/// The meshes, built once.
static MESHES: LazyLock<Vec<(Model, Mesh)>> = LazyLock::new(|| {
	vec![
		(Model::TorusKnot, Mesh::torus_knot(240, 16)),
		(Model::Torus, Mesh::torus(64, 32)),
		(Model::Sphere, Mesh::sphere(32, 64)),
		(Model::Gem, Mesh::gem()),
	]
});

fn mesh_of(model: Model) -> &'static Mesh {
	&MESHES.iter().find(|(candidate, _)| *candidate == model).expect("every model has a mesh").1
}

fn material_of(material: Material) -> RenderMaterial {
	match material {
		Material::Bronze => RenderMaterial::BRONZE,
		Material::Jade => RenderMaterial::JADE,
		Material::Ice => RenderMaterial::ICE,
		Material::Ruby => RenderMaterial::RUBY,
		Material::Clay => RenderMaterial::CLAY,
	}
}

/// A rendered frame, shared by every viewer it is sent to: each connection puts its own header on it.
pub struct Frame {
	kind: FrameKind,
	width: u16,
	height: u16,
	render_us: u32,
	encode_us: u32,
	/// The [`Scene::author`] and [`Scene::seq`] of the scene it shows.
	author: u32,
	scene_seq: u32,
	jpeg: Vec<u8>,
}

impl Frame {
	/// The message for one connection: its own counter, and the change this frame shows if that change
	/// was the connection's own (so that its browser can time the round trip), else 0.
	fn pack(&self, seq: u32, client: Option<u32>) -> Vec<u8> {
		let view_id = if client == Some(self.author) { self.scene_seq } else { 0 };
		FrameHeader {
			seq,
			view_id,
			kind: self.kind,
			width: self.width,
			height: self.height,
			render_us: self.render_us,
			encode_us: self.encode_us,
		}
		.pack(&self.jpeg)
	}
}

type SubId = u64;

/// One connected viewer, as the renderer sees it.
struct Sub {
	id: SubId,
	/// What it asked for; none until its browser says how big its frames are.
	display: Option<Display>,
	/// Changes whenever `display` does, so that a frame made for an older size is not sent.
	generation: u32,
	/// When the display last changed: a new size starts as a change of the picture for this viewer.
	display_at: Instant,
	credits: Arc<Credits>,
	frames: mpsc::Sender<Arc<Frame>>,
	/// The newest picture version and kind it was sent.
	delivered: Option<(u64, FrameKind)>,
}

struct SharedState {
	scene: Scene,
	/// Counts changes of the picture (not of who watches, nor of the turntable's switch).
	version: u64,
	changed_at: Instant,
	/// A task is turning the camera.
	turning: bool,
	/// A task is rendering for the viewers.
	rendering: bool,
	subs: Vec<Sub>,
	next_id: SubId,
}

/// What everyone shares, who is watching, and the one renderer that serves them.
pub struct ViewerHub {
	state: Mutex<SharedState>,
	snapshot: watch::Sender<Utf8Bytes>,
	/// Something the renderer may have to act on: a change of the picture, a viewer coming, going or
	/// resizing, an acknowledgement.
	wake: Notify,
	/// How many renderings were made, for the tests and the log.
	renders: AtomicUsize,
}

impl ViewerHub {
	pub fn new() -> Arc<Self> {
		let state = SharedState {
			scene: Scene::default(),
			version: 1,
			changed_at: Instant::now(),
			turning: false,
			rendering: false,
			subs: Vec::new(),
			next_id: 1,
		};
		let (snapshot, _) = watch::channel(Self::published(&state));
		Arc::new(Self { state: Mutex::new(state), snapshot, wake: Notify::new(), renders: AtomicUsize::new(0) })
	}

	fn lock(&self) -> MutexGuard<'_, SharedState> {
		self.state.lock().unwrap_or_else(PoisonError::into_inner)
	}

	/// What the hub tells its connections, as the text they replicate: the scene, who watches, the models.
	fn published(state: &SharedState) -> Utf8Bytes {
		let models = MESHES
			.iter()
			.map(|(model, mesh)| ModelInfo {
				model: *model,
				name: model.label().to_owned(),
				triangles: mesh.triangles.len() as u32,
			})
			.collect();
		let hub = Hub { viewers: state.subs.len() as u32, models, scene: state.scene };
		serde_json::to_string(&hub).expect("a hub always serializes").into()
	}

	/// Count a viewer in. The subscription counts it out when dropped, whatever ends the connection;
	/// the receiver is where its frames arrive.
	fn join(self: &Arc<Self>) -> (Subscription, mpsc::Receiver<Arc<Frame>>) {
		let credits = Arc::new(Credits::default());
		let (frames, receiver) = mpsc::channel(FRAMES_IN_FLIGHT + 1);
		let mut state = self.lock();
		let id = state.next_id;
		state.next_id += 1;
		state.subs.push(Sub { id, display: None, generation: 0, display_at: Instant::now(), credits: credits.clone(), frames, delivered: None });
		self.snapshot.send_replace(Self::published(&state));
		self.keep_turning(&mut state);
		self.keep_rendering(&mut state);
		(Subscription { hub: self.clone(), id, credits }, receiver)
	}

	fn leave(&self, id: SubId) {
		let mut state = self.lock();
		state.subs.retain(|sub| sub.id != id);
		self.snapshot.send_replace(Self::published(&state));
		self.wake.notify_one();
	}

	/// Someone changed the scene. Everyone is told, unless it changed nothing.
	fn edit(self: &Arc<Self>, patch: &Patch) {
		let mut state = self.lock();
		let before = state.scene;
		if !state.scene.apply(patch) {
			return;
		}
		self.picture_may_have_changed(&mut state, &before);
		self.snapshot.send_replace(Self::published(&state));
		self.keep_turning(&mut state);
	}

	/// The renderer has something to do if what the scene looks like changed.
	fn picture_may_have_changed(&self, state: &mut SharedState, before: &Scene) {
		if !state.scene.same_picture(before) {
			state.version += 1;
			state.changed_at = Instant::now();
			self.wake.notify_one();
		}
	}

	/// Start the turntable if it should be turning and is not.
	fn keep_turning(self: &Arc<Self>, state: &mut SharedState) {
		if state.scene.spin && !state.subs.is_empty() && !state.turning && tokio::runtime::Handle::try_current().is_ok() {
			state.turning = true;
			tokio::spawn(self.clone().turn());
		}
	}

	/// Turn the camera for everyone while the turntable is on and somebody is watching. The scene
	/// then says it was the server (author 0) that changed it.
	async fn turn(self: Arc<Self>) {
		let mut ticks = interval(TURN_EVERY);
		ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
		let mut last = Instant::now();
		loop {
			ticks.tick().await;
			let now = Instant::now();
			let seconds = now.duration_since(last).as_secs_f32();
			last = now;
			let mut state = self.lock();
			if !state.scene.spin || state.subs.is_empty() {
				state.turning = false;
				return;
			}
			let before = state.scene;
			state.scene.advance(seconds);
			state.scene.author = 0;
			self.picture_may_have_changed(&mut state, &before);
			self.snapshot.send_replace(Self::published(&state));
		}
	}

	/// Start the renderer if somebody watches and it is not running.
	fn keep_rendering(self: &Arc<Self>, state: &mut SharedState) {
		if !state.rendering && !state.subs.is_empty() && tokio::runtime::Handle::try_current().is_ok() {
			state.rendering = true;
			tokio::spawn(self.clone().render());
		}
	}

	/// Serve the viewers until none is left: work out who needs which frame, render each picture that
	/// is needed once, and send it to everyone it is for.
	async fn render(self: Arc<Self>) {
		loop {
			let step = self.next_step(Instant::now());
			match step {
				Step::Stop => return,
				Step::Wait(deadline) => {
					// created after the decision, but a wake in between is kept for it by `Notify`
					let woken = self.wake.notified();
					match deadline {
						Some(at) => {
							let _ = timeout_at(tokio::time::Instant::from_std(at), woken).await;
						}
						None => woken.await,
					}
				}
				Step::Render(work) => self.render_and_send(work).await,
			}
		}
	}

	/// What the renderer should do now. A viewer needs a frame if the picture changed since the last
	/// it was sent, or its size did, and it has room for one: a preview while the picture is moving,
	/// the full-quality frame once it has rested. A new or resized viewer counts as the picture having
	/// just changed for it, so it too starts with a preview.
	fn next_step(&self, now: Instant) -> Step {
		let mut state = self.lock();
		if state.subs.is_empty() {
			state.rendering = false;
			return Step::Stop;
		}
		let version = state.version;
		let mut wanted: [Vec<((SubId, u32), Want)>; 2] = [Vec::new(), Vec::new()];
		let mut deadline: Option<Instant> = None;
		for sub in &state.subs {
			let Some(display) = sub.display else { continue };
			let settled_at = state.changed_at.max(sub.display_at) + SETTLE;
			let kind = if now >= settled_at { FrameKind::Final } else { FrameKind::Preview };
			let done = matches!(sub.delivered, Some((seen, sent)) if seen == version && (sent == FrameKind::Final || kind == FrameKind::Preview));
			if done {
				// a preview is out and the full-quality frame is due when the picture has rested
				if kind == FrameKind::Preview {
					deadline = Some(deadline.map_or(settled_at, |at| at.min(settled_at)));
				}
				continue;
			}
			if !sub.credits.has_room() {
				continue; // an acknowledgement wakes the renderer
			}
			let slot = usize::from(kind == FrameKind::Final);
			wanted[slot].push(((sub.id, sub.generation), want(&display, kind)));
		}
		if wanted.iter().all(Vec::is_empty) {
			return Step::Wait(deadline);
		}
		let passes = [FrameKind::Preview, FrameKind::Final]
			.into_iter()
			.zip(wanted)
			.filter(|(_, viewers)| !viewers.is_empty())
			.map(|(kind, viewers)| (kind, share::plan(&viewers)))
			.collect();
		Step::Render(Work { scene: state.scene, version, passes })
	}

	async fn render_and_send(&self, work: Work) {
		let (version, scene) = (work.version, work.scene);
		let outcome = tokio::task::spawn_blocking(move || render_passes(&work)).await;
		let (pictures, renderings) = outcome.unwrap_or_default();
		self.renders.fetch_add(renderings, Ordering::Relaxed);

		let mut state = self.lock();
		for picture in pictures {
			let frame = picture.jpeg.map(|jpeg| {
				Arc::new(Frame {
					kind: picture.kind,
					width: picture.width,
					height: picture.height,
					render_us: picture.render_us,
					encode_us: picture.encode_us,
					author: scene.author,
					scene_seq: scene.seq,
					jpeg,
				})
			});
			for (id, generation) in picture.viewers {
				// a viewer that left, or resized meanwhile, is not sent what was made for before
				let Some(sub) = state.subs.iter_mut().find(|sub| sub.id == id && sub.generation == generation) else { continue };
				// even a failed render marks it done: it would otherwise be retried without end
				sub.delivered = Some((version, picture.kind));
				if let Some(frame) = &frame {
					if sub.frames.try_send(frame.clone()).is_ok() {
						sub.credits.sent();
					}
				}
			}
		}
	}
}

/// What the renderer decided to do: the scene, and for each kind of frame how to serve who needs it.
struct Work {
	scene: Scene,
	version: u64,
	passes: Vec<(FrameKind, Vec<Rendering<(SubId, u32)>>)>,
}

enum Step {
	Stop,
	/// Nothing to do until something wakes the renderer, or this time (when a preview is due to be
	/// followed by the full-quality frame).
	Wait(Option<Instant>),
	Render(Work),
}

/// A viewer's connection to the hub.
struct Subscription {
	hub: Arc<ViewerHub>,
	id: SubId,
	credits: Arc<Credits>,
}

impl Subscription {
	/// The browser says how big its frames are.
	fn set_display(&self, display: Display) {
		let mut state = self.hub.lock();
		if let Some(sub) = state.subs.iter_mut().find(|sub| sub.id == self.id) {
			if sub.display != Some(display) {
				sub.display = Some(display);
				sub.generation = sub.generation.wrapping_add(1);
				sub.display_at = Instant::now();
				sub.delivered = None;
				self.hub.wake.notify_one();
			}
		}
	}

	/// The browser has drawn a frame: it may be sent another.
	fn acknowledge(&self) {
		self.credits.acknowledged();
		self.hub.wake.notify_one();
	}
}

impl Drop for Subscription {
	fn drop(&mut self) {
		self.hub.leave(self.id);
	}
}

/// How many frames are sent and not acknowledged yet.
#[derive(Default)]
struct Credits {
	in_flight: AtomicUsize,
}

impl Credits {
	fn has_room(&self) -> bool {
		self.in_flight.load(Ordering::Acquire) < FRAMES_IN_FLIGHT
	}

	fn sent(&self) {
		self.in_flight.fetch_add(1, Ordering::AcqRel);
	}

	/// An acknowledgement. One that has nothing to acknowledge is ignored, so a client cannot grant
	/// itself more frames in flight than the limit.
	fn acknowledged(&self) {
		let _ = self.in_flight.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
	}
}

pub async fn upgrade(
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

async fn connection(socket: WebSocket, peer: SocketAddr, state: Arc<AppState>) {
	println!("[{peer}] viewer connected");
	let (mut sink, mut incoming) = socket.split();
	let (subscription, mut outgoing) = state.viewer.join();
	let mut hub = state.viewer.snapshot.subscribe();

	let mut clean = false;
	// who this browser says it is, from its patches; its own frames name its own changes
	let mut client: Option<u32> = None;
	let mut seq = 0_u32;
	let current = hub.borrow_and_update().clone();
	if sink.send(Message::Text(current)).await.is_ok() {
		loop {
			tokio::select! {
				changed = hub.changed() => {
					if changed.is_err() {
						break;
					}
					let json = hub.borrow_and_update().clone();
					if sink.send(Message::Text(json)).await.is_err() {
						break;
					}
				}
				frame = outgoing.recv() => {
					let Some(frame) = frame else { break };
					let message = frame.pack(seq, client);
					seq = seq.wrapping_add(1);
					if sink.send(Message::Binary(message.into())).await.is_err() {
						break;
					}
				}
				message = incoming.next() => match classify(message) {
					Incoming::Text(text) => match serde_json::from_str::<ClientMsg>(&text) {
						Ok(ClientMsg::Set(patch)) => {
							client = Some(patch.client.max(1));
							state.viewer.edit(&patch);
						}
						Ok(ClientMsg::Display(display)) => subscription.set_display(display.sanitized()),
						Ok(ClientMsg::Ack { .. }) => subscription.acknowledge(),
						Err(error) => eprintln!("[{peer}] ill-formed viewer message ignored: {error}"),
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
	drop(subscription);
	// completes the closing handshake, see `connection` in main.rs
	let _ = sink.close().await;
	println!("[{peer}] viewer disconnected ({})", how(clean));
}

/// One picture made from a rendering, and who it is for. `jpeg` is `None` if it could not be made.
struct Picture {
	kind: FrameKind,
	width: u16,
	height: u16,
	render_us: u32,
	encode_us: u32,
	jpeg: Option<Vec<u8>>,
	viewers: Vec<(SubId, u32)>,
}

/// CPU work: rasterize each rendering once, reduce it for the outputs that are smaller, compress each
/// output. Returns the pictures and how many renderings were made. The scene is already within bounds.
fn render_passes(work: &Work) -> (Vec<Picture>, usize) {
	let scene = &work.scene;
	let mut pictures = Vec::new();
	let mut renderings = 0;
	for (kind, plan) in &work.passes {
		let supersample = match kind {
			FrameKind::Preview => 1,
			FrameKind::Final => 2,
		};
		for rendering in plan {
			let settings = Settings {
				width: rendering.size.0,
				height: rendering.size.1,
				yaw: scene.yaw,
				pitch: scene.pitch,
				distance: scene.distance,
				light_azimuth: scene.light,
				shininess: scene.shininess,
				wireframe: scene.wireframe,
				supersample,
				material: material_of(scene.material),
			};
			let started = Instant::now();
			let master = moth_sample_render::render(mesh_of(scene.model), &settings);
			let render_us = started.elapsed().as_micros() as u32;
			renderings += 1;

			for output in &rendering.outputs {
				let started = Instant::now();
				let reduced: Image;
				let image = if (master.width, master.height) == output.size {
					&master
				} else {
					reduced = master.resized(output.size.0, output.size.1);
					&reduced
				};
				let jpeg = moth_sample_render::encode_jpeg(image, output.quality).ok();
				let encode_us = started.elapsed().as_micros() as u32;
				pictures.push(Picture {
					kind: *kind,
					width: image.width as u16,
					height: image.height as u16,
					render_us,
					encode_us,
					jpeg,
					viewers: output.viewers.clone(),
				});
			}
		}
	}
	(pictures, renderings)
}

#[cfg(test)]
mod tests {
	use super::*;
	use share::Output;

	fn hub_of(hub: &Arc<ViewerHub>) -> Hub {
		serde_json::from_str::<Hub>(hub.snapshot.borrow().as_str()).unwrap()
	}

	fn patch(client: u32, seq: u32) -> Patch {
		Patch { client, seq, ..Patch::default() }
	}

	fn display(width: u32, height: u32) -> Display {
		Display { width, height, ..Display::default() }
	}

	/// What `render_passes` makes of a rendering for one viewer of the given size.
	fn one_picture(kind: FrameKind, size: (u32, u32)) -> Picture {
		let rendering = Rendering { size, outputs: vec![Output { size, quality: 90, viewers: vec![(1, 0)] }] };
		let work = Work { scene: Scene::default(), version: 1, passes: vec![(kind, vec![rendering])] };
		render_passes(&work).0.remove(0)
	}

	/// The next frame of a viewer, or a failure after a second.
	async fn next(frames: &mut mpsc::Receiver<Arc<Frame>>) -> Arc<Frame> {
		tokio::time::timeout(Duration::from_secs(5), frames.recv()).await.expect("a frame in time").expect("the hub is alive")
	}

	/// Whether nothing arrives for a while.
	async fn quiet(frames: &mut mpsc::Receiver<Arc<Frame>>, millis: u64) -> bool {
		tokio::time::timeout(Duration::from_millis(millis), frames.recv()).await.is_err()
	}

	/// A measurement, not a check: `cargo test -p moth-sample-back-end timing -- --ignored --nocapture`.
	#[tokio::test]
	#[ignore]
	async fn timing_of_a_final_frame() {
		for round in 0..3 {
			let picture = one_picture(FrameKind::Final, (1030, 686));
			println!(
				"round {round}: render {:.1} ms, encode {:.1} ms, {} bytes",
				f64::from(picture.render_us) / 1e3,
				f64::from(picture.encode_us) / 1e3,
				picture.jpeg.unwrap().len()
			);
		}
		// the same rendering reduced, to see what a smaller viewer costs on top
		let rendering = Rendering { size: (1030, 686), outputs: vec![Output { quality: 90, size: (1030, 686), viewers: vec![(1, 0)] }, Output { quality: 90, size: (600, 400), viewers: vec![(2, 0)] }] };
		let work = Work { scene: Scene::default(), version: 1, passes: vec![(FrameKind::Final, vec![rendering])] };
		for round in 0..3 {
			let (pictures, _) = render_passes(&work);
			println!("round {round}: shared rendering {:.1} ms, big encode {:.1} ms, reduced + encode {:.1} ms", f64::from(pictures[0].render_us) / 1e3, f64::from(pictures[0].encode_us) / 1e3, f64::from(pictures[1].encode_us) / 1e3);
		}
	}

	#[test]
	fn every_model_has_a_mesh_and_a_label() {
		for model in Model::ALL {
			assert!(!mesh_of(model).triangles.is_empty());
			assert!(!model.label().is_empty());
		}
	}

	#[test]
	fn a_frame_is_a_decodable_header_and_a_jpeg_of_the_size_it_says() {
		for (kind, size) in [(FrameKind::Preview, (320, 240)), (FrameKind::Final, (640, 480))] {
			let picture = one_picture(kind, size);
			let frame = Frame {
				kind,
				width: picture.width,
				height: picture.height,
				render_us: picture.render_us,
				encode_us: picture.encode_us,
				author: 0,
				scene_seq: 0,
				jpeg: picture.jpeg.unwrap(),
			};
			let message = frame.pack(0, None);
			let (parsed, payload) = FrameHeader::unpack(&message).unwrap();
			assert_eq!((u32::from(parsed.width), u32::from(parsed.height)), size);
			assert_eq!(parsed.kind, kind);
			assert_eq!(&payload[..2], &[0xFF, 0xD8], "a JPEG starts with SOI");
			assert_eq!(&payload[payload.len() - 2..], &[0xFF, 0xD9], "and ends with EOI");
		}
	}

	#[test]
	fn a_rendering_is_reduced_for_a_smaller_output_and_rendered_once() {
		let rendering = Rendering {
			size: (640, 400),
			outputs: vec![Output { quality: 90, size: (640, 400), viewers: vec![(1, 0), (2, 0)] }, Output { quality: 90, size: (320, 200), viewers: vec![(3, 0)] }],
		};
		let work = Work { scene: Scene::default(), version: 1, passes: vec![(FrameKind::Final, vec![rendering])] };
		let (pictures, renderings) = render_passes(&work);
		assert_eq!(renderings, 1, "three viewers, one rasterization");
		assert_eq!(pictures.len(), 2, "two pictures");
		assert_eq!((pictures[0].width, pictures[0].height, pictures[0].viewers.len()), (640, 400, 2));
		assert_eq!((pictures[1].width, pictures[1].height, pictures[1].viewers.len()), (320, 200, 1));
		assert_eq!(pictures[0].render_us, pictures[1].render_us, "the same rendering");
		assert!(pictures[1].jpeg.as_ref().unwrap().len() < pictures[0].jpeg.as_ref().unwrap().len(), "and a smaller file");
	}

	#[test]
	fn a_frame_names_the_change_only_to_the_browser_that_made_it() {
		let frame = Frame { kind: FrameKind::Final, width: 1, height: 1, render_us: 0, encode_us: 0, author: 5, scene_seq: 42, jpeg: vec![] };
		let view_id = |client| FrameHeader::unpack(&frame.pack(0, client)).unwrap().0.view_id;
		assert_eq!(view_id(Some(5)), 42, "its own change: it can time the round trip");
		assert_eq!(view_id(Some(6)), 0, "somebody else's change");
		assert_eq!(view_id(None), 0, "a browser that has not changed anything");
		let turntable = Frame { author: 0, ..frame };
		assert_eq!(FrameHeader::unpack(&turntable.pack(0, Some(5))).unwrap().0.view_id, 0, "the turntable's");
	}

	#[test]
	fn the_viewer_count_follows_joins_and_drops() {
		let hub = ViewerHub::new();
		assert_eq!(hub_of(&hub).viewers, 0);
		let first = hub.join();
		let second = hub.join();
		assert_eq!(hub_of(&hub).viewers, 2);
		drop(first);
		assert_eq!(hub_of(&hub).viewers, 1);
		drop(second);
		assert_eq!(hub_of(&hub).viewers, 0);
		assert_eq!(hub_of(&hub).models.len(), 4);
	}

	#[test]
	fn everyone_sees_one_scene_and_a_patch_from_anyone_changes_it_for_all() {
		let hub = ViewerHub::new();
		let mut first = hub.snapshot.subscribe();
		let mut second = hub.snapshot.subscribe();
		first.borrow_and_update();
		second.borrow_and_update();

		hub.edit(&Patch { material: Some(Material::Jade), ..patch(7, 1) });
		hub.edit(&Patch { yaw: Some(2.0), ..patch(8, 1) }); // a different client, a different field
		for seen in [&mut first, &mut second] {
			assert!(seen.has_changed().unwrap());
			let scene = serde_json::from_str::<Hub>(seen.borrow_and_update().as_str()).unwrap().scene;
			assert_eq!((scene.material, scene.yaw, scene.author), (Material::Jade, 2.0, 8), "both changes, in everyone's copy");
		}
	}

	#[test]
	fn a_patch_that_changes_nothing_is_not_published_and_a_hostile_one_is_bounded() {
		let hub = ViewerHub::new();
		let mut seen = hub.snapshot.subscribe();
		seen.borrow_and_update();
		hub.edit(&patch(3, 1));
		hub.edit(&Patch { model: Some(Scene::default().model), ..patch(3, 2) });
		assert!(!seen.has_changed().unwrap(), "nobody is bothered by an edit that changes nothing");

		hub.edit(&Patch { yaw: Some(f32::NAN), distance: Some(1e30), shininess: Some(f32::INFINITY), ..patch(3, 3) });
		let scene = hub_of(&hub).scene;
		assert_eq!(scene.yaw, Scene::default().yaw);
		assert_eq!(scene.distance, moth_sample_protocol::viewer::MAX_DISTANCE);
		assert!(seen.has_changed().unwrap());
	}

	#[test]
	fn a_viewer_joining_or_leaving_does_not_change_the_scene() {
		let hub = ViewerHub::new();
		hub.edit(&Patch { wireframe: Some(true), ..patch(2, 1) });
		let before = hub_of(&hub).scene;
		drop(hub.join());
		assert_eq!(hub_of(&hub).scene, before, "the scene outlives its viewers: the next one finds it as it was left");
		assert_eq!(hub.lock().version, 2, "and nobody's picture was changed by it");
	}

	#[tokio::test]
	async fn the_turntable_turns_the_camera_for_everyone_while_somebody_watches() {
		let hub = ViewerHub::new();
		let (presence, _frames) = hub.join();
		let start = hub_of(&hub).scene.yaw;
		hub.edit(&Patch { spin: Some(true), ..patch(4, 1) });
		tokio::time::sleep(Duration::from_millis(300)).await;
		let turning = hub_of(&hub).scene;
		assert!(turning.yaw != start, "the camera moved");
		assert_eq!(turning.author, 0, "and it says the server did it, not a client");

		// switching it off stops it
		hub.edit(&Patch { spin: Some(false), ..patch(4, 2) });
		let stopped = hub_of(&hub).scene.yaw;
		tokio::time::sleep(Duration::from_millis(100)).await;
		assert_eq!(hub_of(&hub).scene.yaw, stopped);

		// and it stops by itself when nobody watches, then resumes for the next viewer
		hub.edit(&Patch { spin: Some(true), ..patch(4, 3) });
		drop(presence);
		tokio::time::sleep(Duration::from_millis(100)).await;
		let parked = hub_of(&hub).scene.yaw;
		tokio::time::sleep(Duration::from_millis(100)).await;
		assert_eq!(hub_of(&hub).scene.yaw, parked, "no viewer: it does not turn, not even to publish into the void");
		let _next = hub.join();
		tokio::time::sleep(Duration::from_millis(200)).await;
		assert!(hub_of(&hub).scene.yaw != parked, "a viewer arrived: it turns again");
	}

	#[tokio::test]
	async fn a_viewer_is_sent_a_preview_then_one_full_frame_and_then_nothing() {
		let hub = ViewerHub::new();
		let (viewer, mut frames) = hub.join();
		viewer.set_display(display(320, 240));
		let preview = next(&mut frames).await;
		assert_eq!((preview.kind, preview.width, preview.height), (FrameKind::Preview, 160, 120));
		let full = next(&mut frames).await;
		assert_eq!((full.kind, full.width, full.height), (FrameKind::Final, 320, 240));
		viewer.acknowledge();
		viewer.acknowledge();
		assert!(quiet(&mut frames, 600).await, "nothing changed: nothing is rendered again");
		assert_eq!(hub.renders.load(Ordering::Relaxed), 2);
	}

	#[tokio::test]
	async fn viewers_of_the_same_size_are_sent_the_very_same_frame_from_one_rendering() {
		let hub = ViewerHub::new();
		let (a, mut a_frames) = hub.join();
		let (b, mut b_frames) = hub.join();
		a.set_display(display(320, 240));
		b.set_display(display(320, 240));
		let (a_preview, b_preview) = (next(&mut a_frames).await, next(&mut b_frames).await);
		let (a_full, b_full) = (next(&mut a_frames).await, next(&mut b_frames).await);
		assert!(Arc::ptr_eq(&a_preview, &b_preview) && Arc::ptr_eq(&a_full, &b_full), "the same bytes, shared");
		assert_eq!(hub.renders.load(Ordering::Relaxed), 2, "a preview and a full frame, for both");
	}

	#[tokio::test]
	async fn a_smaller_viewer_of_the_same_shape_is_served_from_the_same_rendering() {
		let hub = ViewerHub::new();
		let (big, mut big_frames) = hub.join();
		let (small, mut small_frames) = hub.join();
		big.set_display(display(640, 400));
		small.set_display(display(320, 200));
		let (big_preview, small_preview) = (next(&mut big_frames).await, next(&mut small_frames).await);
		let (big_full, small_full) = (next(&mut big_frames).await, next(&mut small_frames).await);
		assert_eq!((big_full.width, big_full.height, small_full.width, small_full.height), (640, 400, 320, 200));
		assert_eq!((big_preview.width, small_preview.width), (320, 160), "previews too, each at its size");
		assert_eq!(big_full.render_us, small_full.render_us, "reduced from the one rendering");
		assert!(small_full.jpeg.len() < big_full.jpeg.len());
		assert_eq!(hub.renders.load(Ordering::Relaxed), 2, "one rendering per kind, not per viewer");
	}

	#[tokio::test]
	async fn a_viewer_chooses_how_big_its_previews_are() {
		let hub = ViewerHub::new();
		let (coarse, mut coarse_frames) = hub.join();
		let (sharp, mut sharp_frames) = hub.join();
		coarse.set_display(Display { preview: 0.25, ..display(640, 400) }.sanitized());
		sharp.set_display(Display { preview: 1.0, ..display(640, 400) }.sanitized());
		let (coarse_preview, sharp_preview) = (next(&mut coarse_frames).await, next(&mut sharp_frames).await);
		assert_eq!((coarse_preview.kind, coarse_preview.width, coarse_preview.height), (FrameKind::Preview, 160, 100));
		assert_eq!((sharp_preview.kind, sharp_preview.width, sharp_preview.height), (FrameKind::Preview, 640, 400), "a preview as big as the full frame, for the viewer that wants it");
		let (coarse_full, sharp_full) = (next(&mut coarse_frames).await, next(&mut sharp_frames).await);
		assert_eq!((coarse_full.width, sharp_full.width), (640, 640), "the full frames are the same size: the setting is for previews only");
		assert!(coarse_preview.render_us != sharp_preview.render_us || coarse_preview.jpeg.len() < sharp_preview.jpeg.len(), "and the coarse preview is the cheaper picture");
	}

	#[tokio::test]
	async fn a_viewer_chooses_the_jpeg_quality_of_each_kind_of_frame() {
		let hub = ViewerHub::new();
		let (fine, mut fine_frames) = hub.join();
		let (coarse, mut coarse_frames) = hub.join();
		let (same, mut same_frames) = hub.join();
		let screen = display(640, 400);
		fine.set_display(Display { quality: 100, preview_quality: 100, ..screen }.sanitized());
		coarse.set_display(Display { quality: 10, preview_quality: 10, ..screen }.sanitized());
		same.set_display(screen);
		let previews = [next(&mut fine_frames).await, next(&mut coarse_frames).await, next(&mut same_frames).await];
		let fulls = [next(&mut fine_frames).await, next(&mut coarse_frames).await, next(&mut same_frames).await];
		for frames in [&previews, &fulls] {
			assert!(frames[0].jpeg.len() > frames[2].jpeg.len() && frames[2].jpeg.len() > frames[1].jpeg.len(), "100 > default > 10: {:?}", frames.iter().map(|f| f.jpeg.len()).collect::<Vec<_>>());
			assert!(frames[0].render_us == frames[1].render_us && frames[1].render_us == frames[2].render_us, "from the same rendering");
		}
		assert_eq!(hub.renders.load(Ordering::Relaxed), 2, "one rendering per kind of frame, three qualities on top of it");
	}

	#[tokio::test]
	async fn the_two_kinds_of_frame_each_follow_their_own_quality() {
		let hub = ViewerHub::new();
		let (viewer, mut frames) = hub.join();
		// previews as sharp as it gets at the full frame's size, the full frame as coarse as it gets
		viewer.set_display(Display { preview: 1.0, quality: 10, preview_quality: 100, ..display(320, 200) }.sanitized());
		let (preview, full) = (next(&mut frames).await, next(&mut frames).await);
		assert_eq!((preview.kind, full.kind), (FrameKind::Preview, FrameKind::Final));
		assert_eq!((preview.width, full.width), (320, 320), "the same size, so the files differ by quality alone");
		assert!(preview.jpeg.len() > full.jpeg.len(), "{} against {} bytes", preview.jpeg.len(), full.jpeg.len());
	}

	#[tokio::test]
	async fn a_viewer_of_another_shape_is_rendered_for_separately() {
		let hub = ViewerHub::new();
		let (wide, mut wide_frames) = hub.join();
		let (tall, mut tall_frames) = hub.join();
		wide.set_display(display(640, 400));
		tall.set_display(display(300, 500));
		for _ in 0..2 {
			next(&mut wide_frames).await;
			next(&mut tall_frames).await;
		}
		assert_eq!(hub.renders.load(Ordering::Relaxed), 4, "each shape has its own preview and full frame");
	}

	#[tokio::test]
	async fn a_slow_viewer_never_holds_up_the_others() {
		let hub = ViewerHub::new();
		let (slow, mut slow_frames) = hub.join();
		let (quick, mut quick_frames) = hub.join();
		slow.set_display(display(320, 240));
		quick.set_display(display(320, 240));
		hub.edit(&Patch { yaw: Some(1.0), ..patch(1, 1) });
		// the slow viewer reads nothing and acknowledges nothing: it gets two frames and no more
		let mut received = 0;
		for step in 0..12 {
			hub.edit(&Patch { yaw: Some(1.0 + step as f32 * 0.1), ..patch(1, 2 + step) });
			tokio::time::sleep(Duration::from_millis(60)).await;
			while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(1), quick_frames.recv()).await {
				received += 1;
				quick.acknowledge();
			}
		}
		assert!(received >= 6, "the quick viewer kept being served ({received} frames)");
		let mut stuck = 0;
		while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(1), slow_frames.recv()).await {
			stuck += 1;
		}
		assert_eq!(stuck, FRAMES_IN_FLIGHT, "while the one that never acknowledges stopped at the limit");

		// when it does acknowledge it is sent the newest state, not the ones it missed
		hub.edit(&Patch { yaw: Some(3.0), ..patch(1, 99) });
		slow.acknowledge();
		slow.acknowledge();
		let latest = next(&mut slow_frames).await;
		assert_eq!(latest.scene_seq, 99, "the newest scene");
	}

	#[tokio::test]
	async fn a_burst_of_changes_is_rendered_far_fewer_times_and_ends_on_the_newest() {
		let hub = ViewerHub::new();
		let (viewer, mut frames) = hub.join();
		viewer.set_display(display(320, 240));
		let mut last = next(&mut frames).await;
		viewer.acknowledge();
		for step in 0..60 {
			hub.edit(&Patch { yaw: Some(1.0 + step as f32 * 0.05), ..patch(1, 100 + step) });
		}
		let mut count = 0;
		while let Ok(Some(frame)) = tokio::time::timeout(Duration::from_millis(800), frames.recv()).await {
			viewer.acknowledge();
			count += 1;
			last = frame;
		}
		assert!(count <= 8, "{count} frames for 60 changes");
		assert_eq!((last.kind, last.scene_seq), (FrameKind::Final, 159), "and the last is the full-quality newest state");
	}

	#[tokio::test]
	async fn a_viewer_that_resizes_is_sent_frames_of_its_new_size_only() {
		let hub = ViewerHub::new();
		let (viewer, mut frames) = hub.join();
		viewer.set_display(display(320, 240));
		next(&mut frames).await;
		next(&mut frames).await;
		viewer.acknowledge();
		viewer.acknowledge();
		viewer.set_display(display(200, 150));
		let preview = next(&mut frames).await;
		assert_eq!((preview.kind, preview.width), (FrameKind::Preview, 100), "a resized viewer starts with a preview again");
		assert_eq!(next(&mut frames).await.width, 200);
	}

	#[tokio::test]
	async fn the_renderer_stops_when_the_last_viewer_leaves_and_starts_for_the_next() {
		let hub = ViewerHub::new();
		let (viewer, mut frames) = hub.join();
		viewer.set_display(display(160, 120));
		next(&mut frames).await;
		drop(viewer);
		tokio::time::sleep(Duration::from_millis(100)).await;
		assert!(!hub.lock().rendering, "nobody is left to render for");
		let (again, mut frames) = hub.join();
		again.set_display(display(160, 120));
		assert_eq!(next(&mut frames).await.kind, FrameKind::Preview, "and it is back for the next viewer");
	}

	#[tokio::test]
	async fn credits_limit_the_frames_in_flight_and_ignore_acknowledgements_nobody_earned() {
		let credits = Credits::default();
		credits.acknowledged(); // nothing in flight: ignored, must not underflow or add room
		for _ in 0..FRAMES_IN_FLIGHT {
			assert!(credits.has_room());
			credits.sent();
		}
		assert!(!credits.has_room(), "no room while the limit is reached");

		credits.acknowledged();
		assert!(credits.has_room(), "an acknowledgement makes room");

		// a flood of acknowledgements does not grant more than the limit
		for _ in 0..1000 {
			credits.acknowledged();
		}
		for _ in 0..FRAMES_IN_FLIGHT {
			credits.sent();
		}
		assert!(!credits.has_room());
	}
}
