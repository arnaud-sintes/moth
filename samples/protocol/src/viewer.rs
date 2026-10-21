//! The remote 3D viewer: the browser sends changes to a shared scene, the server renders the object
//! and streams JPEG frames back.
//!
//! Everyone connected to a server shares one [`Scene`], like the todo list is shared: it lives on the
//! server, and a change made by one browser is seen by all. (To work apart, run another server.)
//!
//! - client to server (text, JSON): [`ClientMsg`], a [`Patch`] to the scene, this browser's [`Display`],
//!   or an acknowledgement;
//! - server to client (text, JSON): the [`Hub`] with the scene, replicated like the todo model;
//! - server to client (binary): a frame, a [`FrameHeader`] followed by the JPEG.
//!
//! What clients send is untrusted: [`Scene::apply`] and [`Display::sanitized`] bound it.

use std::f32::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

pub const MAX_WIDTH: u32 = 1280;
pub const MAX_HEIGHT: u32 = 720;
pub const MIN_DISTANCE: f32 = 2.6;
pub const MAX_DISTANCE: f32 = 14.0;
/// Frames the server may have sent that the client has not acknowledged yet.
pub const FRAMES_IN_FLIGHT: usize = 2;
/// The range of JPEG quality a viewer may ask for (below 10 a picture is not worth sending).
pub const MIN_QUALITY: u32 = 10;
pub const MAX_QUALITY: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
	TorusKnot,
	Torus,
	Sphere,
	Gem,
}

impl Model {
	pub const ALL: [Model; 4] = [Model::TorusKnot, Model::Torus, Model::Sphere, Model::Gem];

	pub fn label(self) -> &'static str {
		match self {
			Model::TorusKnot => "Torus knot",
			Model::Torus => "Torus",
			Model::Sphere => "Sphere",
			Model::Gem => "Faceted gem",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
	Bronze,
	Jade,
	Ice,
	Ruby,
	Clay,
}

impl Material {
	pub const ALL: [Material; 5] =
		[Material::Bronze, Material::Jade, Material::Ice, Material::Ruby, Material::Clay];

	pub fn label(self) -> &'static str {
		match self {
			Material::Bronze => "Bronze",
			Material::Jade => "Jade",
			Material::Ice => "Ice",
			Material::Ruby => "Ruby",
			Material::Clay => "Clay",
		}
	}
}

/// The turntable's speed while auto-rotate is on, in radians per second.
pub const SPIN_RATE: f32 = 0.7;

/// What every viewer of this server sees and steers together: the object, how it is lit and where
/// the camera is. It lives on the server and is replicated to everyone in the [`Hub`]; clients change
/// it with a [`Patch`]. What depends on one browser alone (its canvas size, its resolution setting)
/// is a [`Display`] and is not shared.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Scene {
	pub model: Model,
	pub material: Material,
	/// Camera, orbiting the object: radians around the vertical axis, radians above the horizon, and
	/// the distance from the centre.
	pub yaw: f32,
	pub pitch: f32,
	pub distance: f32,
	/// Key light, in radians around the camera's vertical axis.
	pub light: f32,
	pub shininess: f32,
	pub wireframe: bool,
	/// The turntable: the server turns the camera for everyone while this is on.
	pub spin: bool,
	/// Who made the last change: the [`Patch::client`] that sent it, or 0 for the server (the
	/// turntable). With `seq`, it lets a client tell the echo of its own older edits (to be ignored)
	/// from news.
	pub author: u32,
	pub seq: u32,
}

impl Default for Scene {
	fn default() -> Self {
		Scene {
			model: Model::TorusKnot,
			material: Material::Bronze,
			yaw: 0.6,
			pitch: 0.35,
			distance: 6.3,
			light: 0.7,
			shininess: 48.0,
			wireframe: false,
			spin: false,
			author: 0,
			seq: 0,
		}
	}
}

/// A change to the [`Scene`]: only the fields that are set are changed, so two people changing
/// different things at once both get what they asked for; for the same field the later one wins.
/// Untrusted: [`Scene::apply`] bounds every number.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Patch {
	/// Who sends it (any number but 0, chosen by the browser), and its own counter for its patches.
	pub client: u32,
	pub seq: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub model: Option<Model>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub material: Option<Material>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub yaw: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pitch: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub distance: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub light: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub shininess: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub wireframe: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub spin: Option<bool>,
}

impl Patch {
	/// Whether it changes anything at all.
	pub fn is_empty(&self) -> bool {
		let Patch { client: _, seq: _, model, material, yaw, pitch, distance, light, shininess, wireframe, spin } = self;
		model.is_none()
			&& material.is_none()
			&& yaw.is_none()
			&& pitch.is_none()
			&& distance.is_none()
			&& light.is_none()
			&& shininess.is_none()
			&& wireframe.is_none()
			&& spin.is_none()
	}
}

impl Scene {
	/// Make the change. Every number is made finite and brought within bounds (one that is not a
	/// number is ignored), so that whatever a client sends can neither panic the renderer nor make it
	/// allocate without limit. Returns whether the scene changed; if it did, it now names the patch's
	/// author and counter, if it did not nothing is touched.
	pub fn apply(&mut self, patch: &Patch) -> bool {
		let finite = |value: Option<f32>| value.filter(|value| value.is_finite());
		let mut next = *self;
		next.model = patch.model.unwrap_or(next.model);
		next.material = patch.material.unwrap_or(next.material);
		if let Some(yaw) = finite(patch.yaw) {
			next.yaw = yaw.rem_euclid(TAU);
		}
		if let Some(pitch) = finite(patch.pitch) {
			next.pitch = pitch.clamp(-1.5, 1.5);
		}
		if let Some(distance) = finite(patch.distance) {
			next.distance = distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
		}
		if let Some(light) = finite(patch.light) {
			next.light = light.clamp(-PI, PI);
		}
		if let Some(shininess) = finite(patch.shininess) {
			next.shininess = shininess.clamp(2.0, 200.0);
		}
		next.wireframe = patch.wireframe.unwrap_or(next.wireframe);
		next.spin = patch.spin.unwrap_or(next.spin);
		if next == *self {
			return false;
		}
		*self = Scene { author: patch.client.max(1), seq: patch.seq, ..next };
		true
	}

	/// The turntable advanced by `seconds` (a no-op for a time that is not a sensible one).
	pub fn advance(&mut self, seconds: f32) {
		if seconds.is_finite() && seconds > 0.0 {
			self.yaw = (self.yaw + SPIN_RATE * seconds.min(1.0)).rem_euclid(TAU);
		}
	}

	/// Whether the two scenes look the same: who made them, and whether the turntable is on (it moves
	/// the camera, which is a change of its own), make no difference to a picture.
	pub fn same_picture(&self, other: &Scene) -> bool {
		Scene { author: 0, seq: 0, spin: false, ..*self } == Scene { author: 0, seq: 0, spin: false, ..*other }
	}
}

/// What one browser needs for itself, and tells the server: how big its frames are and how finely
/// they are rendered. Missing fields take their defaults.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Display {
	/// Size of the frame, in pixels; the client scales it to its canvas.
	pub width: u32,
	pub height: u32,
	/// Fraction (0.25 to 1) of `width` x `height` to render the full-quality frame at.
	pub scale: f32,
	/// Fraction (0.1 to 1) of the full-quality frame's size that the previews, sent while the scene is
	/// moving, are rendered at: the browser's choice between a sharper picture and a quicker one while
	/// the user is manipulating it.
	pub preview: f32,
	/// JPEG quality (10 to 100) of the full-quality frame, and of the previews: what a browser spends
	/// in bandwidth for sharpness, separately for when the picture rests and for when it moves.
	pub quality: u32,
	pub preview_quality: u32,
}

impl Default for Display {
	fn default() -> Self {
		Display { width: 640, height: 480, scale: 1.0, preview: 0.5, quality: 90, preview_quality: 60 }
	}
}

impl Display {
	/// What the server will actually render at: within bounds whatever the client said. A size that is
	/// too big is brought inside the biggest frame the server makes by shrinking both sides alike: the
	/// page draws the frame stretched to its canvas, so a frame of another shape would look distorted
	/// (clamping each side on its own did that to tall and very wide windows).
	pub fn sanitized(&self) -> Display {
		let default = Display::default();
		let scale = if self.scale.is_finite() { self.scale.clamp(0.25, 1.0) } else { default.scale };
		let preview = if self.preview.is_finite() { self.preview.clamp(0.1, 1.0) } else { default.preview };
		let quality = self.quality.clamp(MIN_QUALITY, MAX_QUALITY);
		let preview_quality = self.preview_quality.clamp(MIN_QUALITY, MAX_QUALITY);
		let (width, height) = (self.width.max(1), self.height.max(1));
		let fit = (f64::from(MAX_WIDTH) / f64::from(width)).min(f64::from(MAX_HEIGHT) / f64::from(height)).min(1.0);
		let side = |side: u32| ((f64::from(side) * fit).round() as u32).max(1);
		Display { width: side(width).min(MAX_WIDTH), height: side(height).min(MAX_HEIGHT), scale, preview, quality, preview_quality }
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
	/// Change the shared scene.
	Set(Patch),
	/// This browser's own frame size and quality. Only the latest matters: the server skips what was
	/// superseded.
	Display(Display),
	/// The frame `seq` has been drawn; the server may send another.
	Ack { seq: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
	pub model: Model,
	pub name: String,
	pub triangles: u32,
}

/// What the server tells every viewer, replicated like the todo model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hub {
	/// Browsers connected to the viewer right now, this one included.
	pub viewers: u32,
	pub models: Vec<ModelInfo>,
	/// The scene all of them are looking at.
	pub scene: Scene,
}

// ---- frames

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
	/// Quick and coarse, sent while the camera moves.
	Preview,
	/// Full size and anti-aliased, sent once the camera has rested.
	Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
	pub seq: u32,
	/// The [`Scene::seq`] this frame shows when the scene's last change was the receiver's own, else 0.
	pub view_id: u32,
	pub kind: FrameKind,
	pub width: u16,
	pub height: u16,
	pub render_us: u32,
	pub encode_us: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
	TooShort,
	BadMagic,
	BadKind(u8),
}

impl std::fmt::Display for FrameError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FrameError::TooShort => f.write_str("frame shorter than its header"),
			FrameError::BadMagic => f.write_str("not a viewer frame"),
			FrameError::BadKind(kind) => write!(f, "unknown frame kind {kind}"),
		}
	}
}

impl FrameHeader {
	/// `"MOTH"`, so a stray binary message is recognised as not being a frame.
	pub const MAGIC: [u8; 4] = *b"MOTH";
	pub const SIZE: usize = 26;

	/// The message: this header followed by the JPEG. Little-endian.
	pub fn pack(&self, jpeg: &[u8]) -> Vec<u8> {
		let mut message = Vec::with_capacity(Self::SIZE + jpeg.len());
		message.extend_from_slice(&Self::MAGIC);
		message.extend_from_slice(&self.seq.to_le_bytes());
		message.extend_from_slice(&self.view_id.to_le_bytes());
		message.push(match self.kind {
			FrameKind::Preview => 0,
			FrameKind::Final => 1,
		});
		message.push(0); // reserved
		message.extend_from_slice(&self.width.to_le_bytes());
		message.extend_from_slice(&self.height.to_le_bytes());
		message.extend_from_slice(&self.render_us.to_le_bytes());
		message.extend_from_slice(&self.encode_us.to_le_bytes());
		message.extend_from_slice(jpeg);
		message
	}

	/// The header and the JPEG that follows it.
	pub fn unpack(message: &[u8]) -> Result<(FrameHeader, &[u8]), FrameError> {
		if message.len() < Self::SIZE {
			return Err(FrameError::TooShort);
		}
		if message[..4] != Self::MAGIC {
			return Err(FrameError::BadMagic);
		}
		let u32_at = |at: usize| u32::from_le_bytes(message[at..at + 4].try_into().expect("4 bytes"));
		let u16_at = |at: usize| u16::from_le_bytes(message[at..at + 2].try_into().expect("2 bytes"));
		let kind = match message[12] {
			0 => FrameKind::Preview,
			1 => FrameKind::Final,
			other => return Err(FrameError::BadKind(other)),
		};
		let header = FrameHeader {
			seq: u32_at(4),
			view_id: u32_at(8),
			kind,
			width: u16_at(14),
			height: u16_at(16),
			render_us: u32_at(18),
			encode_us: u32_at(22),
		};
		Ok((header, &message[Self::SIZE..]))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn patch(client: u32, seq: u32) -> Patch {
		Patch { client, seq, ..Patch::default() }
	}

	#[test]
	fn the_wire_format_of_the_messages_is_plain_json() {
		let set = ClientMsg::Set(Patch { model: Some(Model::Gem), yaw: Some(1.5), ..patch(9, 3) });
		let json = serde_json::to_string(&set).unwrap();
		assert_eq!(json, r#"{"type":"set","client":9,"seq":3,"model":"gem","yaw":1.5}"#, "unset fields are not sent");
		assert_eq!(serde_json::from_str::<ClientMsg>(&json).unwrap(), set);

		let display = ClientMsg::Display(Display { width: 800, height: 600, scale: 0.5, preview: 0.25, quality: 80, preview_quality: 40 });
		let json = serde_json::to_string(&display).unwrap();
		assert_eq!(json, r#"{"type":"display","width":800,"height":600,"scale":0.5,"preview":0.25,"quality":80,"preview_quality":40}"#);
		assert_eq!(serde_json::from_str::<ClientMsg>(&json).unwrap(), display);

		assert_eq!(serde_json::to_string(&ClientMsg::Ack { seq: 3 }).unwrap(), r#"{"type":"ack","seq":3}"#);
		assert!(serde_json::from_str::<ClientMsg>(r#"{"type":"nope"}"#).is_err());
		assert!(serde_json::from_str::<ClientMsg>(r#"{"type":"set","model":"teapot"}"#).is_err());
	}

	#[test]
	fn missing_fields_take_their_defaults_and_the_hub_carries_the_scene() {
		match serde_json::from_str::<ClientMsg>(r#"{"type":"display","width":900}"#).unwrap() {
			ClientMsg::Display(display) => assert_eq!(display, Display { width: 900, ..Display::default() }),
			other => panic!("{other:?}"),
		}
		let hub: Hub = serde_json::from_str(r#"{"viewers":2,"scene":{"yaw":1.0}}"#).unwrap();
		assert_eq!((hub.viewers, hub.scene), (2, Scene { yaw: 1.0, ..Scene::default() }));
		let hub = Hub { viewers: 1, models: vec![], scene: Scene { spin: true, ..Scene::default() } };
		assert_eq!(serde_json::from_str::<Hub>(&serde_json::to_string(&hub).unwrap()).unwrap(), hub);
	}

	#[test]
	fn a_patch_changes_only_what_it_names_and_signs_the_scene() {
		let mut scene = Scene::default();
		assert!(scene.apply(&Patch { material: Some(Material::Ruby), ..patch(5, 1) }));
		assert_eq!((scene.material, scene.author, scene.seq), (Material::Ruby, 5, 1));
		assert_eq!(Scene { material: Material::Bronze, author: 0, seq: 0, ..scene }, Scene::default(), "nothing else moved");

		// another client changes something else: both changes are there
		assert!(scene.apply(&Patch { yaw: Some(2.0), ..patch(6, 1) }));
		assert_eq!((scene.material, scene.yaw, scene.author), (Material::Ruby, 2.0, 6));
		// the same field again: the later one wins
		assert!(scene.apply(&Patch { yaw: Some(3.0), ..patch(5, 2) }));
		assert_eq!((scene.yaw, scene.author, scene.seq), (3.0, 5, 2));
	}

	#[test]
	fn a_patch_that_changes_nothing_changes_nothing() {
		let mut scene = Scene::default();
		let before = scene;
		assert!(!scene.apply(&patch(5, 1)), "an empty patch");
		assert!(!scene.apply(&Patch { model: Some(scene.model), yaw: Some(scene.yaw), ..patch(5, 2) }), "the same values");
		assert_eq!(scene, before, "not even who made the last change");
		assert!(patch(1, 1).is_empty() && !Patch { spin: Some(false), ..patch(1, 1) }.is_empty());
	}

	#[test]
	fn a_hostile_patch_leaves_the_scene_inside_the_bounds() {
		let mut scene = Scene::default();
		scene.apply(&Patch {
			yaw: Some(f32::NAN),
			pitch: Some(f32::INFINITY),
			distance: Some(-3.0),
			light: Some(f32::NEG_INFINITY),
			shininess: Some(1e30),
			..patch(0, u32::MAX)
		});
		let default = Scene::default();
		assert_eq!(scene.yaw, default.yaw, "not a number: ignored");
		assert_eq!(scene.pitch, default.pitch);
		assert_eq!(scene.light, default.light);
		assert_eq!(scene.distance, MIN_DISTANCE);
		assert_eq!(scene.shininess, 200.0);
		assert_eq!((scene.author, scene.seq), (1, u32::MAX), "client 0 is the server's: nobody else can be it");

		scene.apply(&Patch { yaw: Some(100.0), pitch: Some(99.0), distance: Some(1e9), light: Some(50.0), ..patch(2, 1) });
		assert!((0.0..TAU).contains(&scene.yaw) && (scene.yaw - 100.0_f32.rem_euclid(TAU)).abs() < 1e-3);
		assert_eq!((scene.pitch, scene.distance, scene.light), (1.5, MAX_DISTANCE, PI));
		scene.apply(&Patch { yaw: Some(-0.5), ..patch(2, 2) });
		assert!((scene.yaw - (TAU - 0.5)).abs() < 1e-4);
	}

	#[test]
	fn the_turntable_turns_the_same_amount_per_second_whatever_the_tick() {
		let (mut coarse, mut fine) = (Scene::default(), Scene::default());
		coarse.advance(1.0);
		for _ in 0..100 {
			fine.advance(0.01);
		}
		assert!((coarse.yaw - fine.yaw).abs() < 1e-4);
		assert!((coarse.yaw - (Scene::default().yaw + SPIN_RATE)).abs() < 1e-5);
		let mut scene = Scene::default();
		scene.advance(f32::NAN);
		scene.advance(-5.0);
		scene.advance(f32::INFINITY);
		assert_eq!(scene, Scene::default(), "a time that is not a sensible one does nothing");
		scene.advance(1e6);
		assert!((0.0..TAU).contains(&scene.yaw), "and a very long pause counts as one second at most");
	}

	#[test]
	fn two_scenes_are_the_same_picture_whoever_made_them() {
		let a = Scene { author: 3, seq: 9, ..Scene::default() };
		assert!(a.same_picture(&Scene::default()));
		assert!(!a.same_picture(&Scene { wireframe: true, ..Scene::default() }));
		assert!(a.same_picture(&Scene { spin: true, ..Scene::default() }), "the turntable being on is not a different picture");
		assert!(!a.same_picture(&Scene { yaw: 1.0, ..Scene::default() }), "but the camera it moves is");
	}

	#[test]
	fn a_display_keeps_the_shape_of_the_canvas_it_was_asked_for() {
		// the page draws the frame stretched to its canvas: a frame of another shape looks distorted
		let shape = |display: Display| f64::from(display.width) / f64::from(display.height);
		for (width, height) in [(1030, 686), (560, 1400), (500, 1800), (1900, 500), (3000, 600), (4000, 2250), (800, 800), (300, 5000)] {
			let asked = Display { width, height, ..Display::default() };
			let given = asked.sanitized();
			assert!(given.width <= MAX_WIDTH && given.height <= MAX_HEIGHT, "{asked:?} -> {given:?}");
			let off = (shape(given) / shape(asked)).ln().abs();
			assert!(off < 0.01, "{asked:?} -> {given:?}: the shape changed by {off}");
		}
		let of = |width, height| Display { width, height, ..Display::default() };
		assert_eq!(of(1920, 1080).sanitized(), of(1280, 720));
		assert_eq!(of(560, 1400).sanitized(), of(288, 720));
	}

	#[test]
	fn a_sanitized_display_is_always_inside_the_bounds() {
		let hostile = Display { width: u32::MAX, height: 0, scale: f32::NAN, preview: f32::NAN, ..Display::default() }.sanitized();
		assert_eq!((hostile.width, hostile.height, hostile.scale, hostile.preview), (MAX_WIDTH, 1, 1.0, 0.5), "nothing is zero, and a hostile shape is kept as far as pixels allow");
		assert_eq!(Display { width: 1, height: u32::MAX, scale: 1e9, ..Display::default() }.sanitized(), Display { width: 1, height: MAX_HEIGHT, ..Display::default() });
		let both = Display { width: u32::MAX, height: u32::MAX, scale: f32::INFINITY, ..Display::default() }.sanitized();
		assert_eq!((both.width, both.height, both.scale), (MAX_HEIGHT, MAX_HEIGHT, 1.0), "equal sides stay equal");
		assert_eq!(Display { scale: -4.0, ..Display::default() }.sanitized().scale, 0.25);
		let sensible = Display { width: 800, height: 600, scale: 0.5, preview: 0.25, quality: 75, preview_quality: 35 };
		assert_eq!(sensible.sanitized(), sensible);
		assert_eq!(hostile.sanitized(), hostile, "idempotent");
	}

	#[test]
	fn the_size_of_previews_is_the_viewers_choice_within_bounds() {
		let preview = |preview| Display { preview, ..Display::default() }.sanitized().preview;
		assert_eq!(preview(0.5), 0.5);
		assert_eq!(preview(0.25), 0.25);
		assert_eq!((preview(0.0), preview(-3.0)), (0.1, 0.1), "never so small as to be useless");
		assert_eq!((preview(1.0), preview(40.0)), (1.0, 1.0), "never bigger than the full frame");
		assert_eq!((preview(f32::NAN), preview(f32::INFINITY)), (0.5, 0.5), "not a number: the default");
	}

	#[test]
	fn the_jpeg_quality_is_the_viewers_choice_within_bounds_for_each_kind_of_frame() {
		let given = |quality, preview_quality| {
			let display = Display { quality, preview_quality, ..Display::default() }.sanitized();
			(display.quality, display.preview_quality)
		};
		assert_eq!(given(90, 60), (90, 60));
		assert_eq!(given(100, 10), (100, 10), "both ends are allowed");
		assert_eq!(given(0, 0), (MIN_QUALITY, MIN_QUALITY), "never so low as to be useless");
		assert_eq!(given(101, u32::MAX), (MAX_QUALITY, MAX_QUALITY), "and never above what JPEG has");
		assert_eq!((Display::default().quality, Display::default().preview_quality), (90, 60), "full frames are finer than previews by default");
		let missing: ClientMsg = serde_json::from_str(r#"{"type":"display","width":900}"#).unwrap();
		assert!(matches!(missing, ClientMsg::Display(d) if d.quality == 90 && d.preview_quality == 60), "a client that does not say gets the defaults");
	}

	fn header() -> FrameHeader {
		FrameHeader { seq: 0xA1B2C3D4, view_id: 77, kind: FrameKind::Final, width: 1280, height: 720, render_us: 12_345, encode_us: 6_789 }
	}

	#[test]
	fn a_frame_survives_packing_and_unpacking() {
		let jpeg = [0xFF, 0xD8, 1, 2, 3, 0xFF, 0xD9];
		let message = header().pack(&jpeg);
		assert_eq!(message.len(), FrameHeader::SIZE + jpeg.len());
		let (parsed, rest) = FrameHeader::unpack(&message).unwrap();
		assert_eq!(parsed, header());
		assert_eq!(rest, jpeg);

		let preview = FrameHeader { kind: FrameKind::Preview, ..header() };
		assert_eq!(FrameHeader::unpack(&preview.pack(&[])).unwrap().0, preview);
	}

	#[test]
	fn the_header_layout_is_fixed() {
		// the browser reads these offsets by hand: they are part of the protocol
		let message = header().pack(&[]);
		assert_eq!(&message[..4], b"MOTH");
		assert_eq!(u32::from_le_bytes(message[4..8].try_into().unwrap()), 0xA1B2C3D4);
		assert_eq!(u32::from_le_bytes(message[8..12].try_into().unwrap()), 77);
		assert_eq!(message[12], 1);
		assert_eq!(u16::from_le_bytes(message[14..16].try_into().unwrap()), 1280);
		assert_eq!(u16::from_le_bytes(message[16..18].try_into().unwrap()), 720);
		assert_eq!(u32::from_le_bytes(message[18..22].try_into().unwrap()), 12_345);
		assert_eq!(u32::from_le_bytes(message[22..26].try_into().unwrap()), 6_789);
	}

	#[test]
	fn what_is_not_a_frame_is_refused_not_trusted() {
		assert_eq!(FrameHeader::unpack(&[]), Err(FrameError::TooShort));
		assert_eq!(FrameHeader::unpack(&header().pack(&[])[..25]), Err(FrameError::TooShort));
		let mut message = header().pack(&[1, 2]);
		message[0] = b'X';
		assert_eq!(FrameHeader::unpack(&message), Err(FrameError::BadMagic));
		let mut message = header().pack(&[1, 2]);
		message[12] = 9;
		assert_eq!(FrameHeader::unpack(&message), Err(FrameError::BadKind(9)));
	}
}
