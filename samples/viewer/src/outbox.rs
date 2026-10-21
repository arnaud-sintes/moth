//! What this browser has yet to tell the server about the shared scene, and which of the server's
//! news to believe: pure bookkeeping, no browser.
//!
//! The scene is shared by everyone connected, so the server's copy is the truth and this page only
//! *asks* for changes, in a [`Patch`] that holds just what changed. The server tells everybody the
//! new scene, and each scene names who changed it last ([`Scene::author`], [`Scene::seq`]), which is
//! how the page tells the echo of its own older edits from news:
//!
//! - an echo of an edit that has been superseded by a newer one already sent is stale: believing it
//!   would pull the camera back while the user is still dragging it;
//! - while something is waiting to be sent, an echo of the page's own earlier edit is stale as well,
//!   for the same reason;
//! - anything else (somebody else's change, the turntable's, the echo of the newest own edit, which
//!   also settles races) is the truth and is shown, controls included.

use moth_sample_protocol::viewer::{Patch, Scene};

use crate::orbit::Orbit;

pub struct Outbox {
	client: u32,
	next_seq: u32,
	/// The counter of the newest patch sent on this connection (0: none).
	latest_sent: u32,
	pending: Patch,
	camera: bool,
}

impl Outbox {
	/// `client` names this page in the patches it sends (0 is the server's, so it is made 1).
	pub fn new(client: u32) -> Self {
		Outbox { client: client.max(1), next_seq: 1, latest_sent: 0, pending: Patch::default(), camera: false }
	}

	/// Ask for something. Asking again for the same field before the patch is sent replaces it.
	pub fn request(&mut self, change: impl FnOnce(&mut Patch)) {
		change(&mut self.pending);
	}

	/// The camera was moved; its position is taken when the patch is made, so that however fast the
	/// pointer moves only the position of that moment is sent.
	pub fn camera_moved(&mut self) {
		self.camera = true;
	}

	/// Whether something waits to be sent.
	pub fn is_waiting(&self) -> bool {
		self.camera || !self.pending.is_empty()
	}

	/// The patch to send now, if there is something to say. It is only a proposal: [`Outbox::sent`]
	/// says it went out.
	pub fn next(&self, camera: Orbit) -> Option<Patch> {
		if !self.is_waiting() {
			return None;
		}
		let mut patch = Patch { client: self.client, seq: self.next_seq, ..self.pending.clone() };
		if self.camera {
			(patch.yaw, patch.pitch, patch.distance) = (Some(camera.yaw), Some(camera.pitch), Some(camera.distance));
		}
		Some(patch)
	}

	/// The patch from [`Outbox::next`] has been sent.
	pub fn sent(&mut self, patch: &Patch) {
		self.latest_sent = patch.seq;
		self.next_seq = patch.seq.wrapping_add(1).max(1);
		self.pending = Patch::default();
		self.camera = false;
	}

	/// Whether to show `scene`, which the server just announced.
	pub fn believe(&self, scene: &Scene) -> bool {
		!(scene.author == self.client && (scene.seq < self.latest_sent || self.is_waiting()))
	}

	/// A new connection: what was said to the old one is gone, and the server's scene (which another
	/// viewer may have changed meanwhile, or a restarted server reset) is the truth.
	pub fn reconnected(&mut self) {
		self.latest_sent = 0;
		self.pending = Patch::default();
		self.camera = false;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const HOME: Orbit = Orbit::HOME;

	fn scene(author: u32, seq: u32) -> Scene {
		Scene { author, seq, ..Scene::default() }
	}

	#[test]
	fn nothing_to_say_sends_nothing() {
		let outbox = Outbox::new(7);
		assert!(!outbox.is_waiting());
		assert_eq!(outbox.next(HOME), None);
	}

	#[test]
	fn a_patch_holds_only_what_was_asked_for_and_a_later_ask_for_a_field_replaces_the_earlier() {
		let mut outbox = Outbox::new(7);
		outbox.request(|patch| patch.wireframe = Some(true));
		outbox.request(|patch| patch.light = Some(1.0));
		outbox.request(|patch| patch.light = Some(2.0));
		let patch = outbox.next(HOME).unwrap();
		assert_eq!((patch.client, patch.seq), (7, 1));
		assert_eq!((patch.wireframe, patch.light, patch.model, patch.yaw), (Some(true), Some(2.0), None, None));
	}

	#[test]
	fn the_camera_is_read_when_the_patch_is_made() {
		let mut outbox = Outbox::new(7);
		outbox.camera_moved();
		outbox.camera_moved();
		let moved = Orbit { yaw: 2.5, pitch: -0.2, distance: 9.0 };
		let patch = outbox.next(moved).unwrap();
		assert_eq!((patch.yaw, patch.pitch, patch.distance), (Some(2.5), Some(-0.2), Some(9.0)));
		assert_eq!(patch.model, None, "and nothing else rides along");
	}

	#[test]
	fn a_patch_is_only_forgotten_once_sent_and_the_counter_moves_on() {
		let mut outbox = Outbox::new(7);
		outbox.request(|patch| patch.spin = Some(true));
		let first = outbox.next(HOME).unwrap();
		assert_eq!(outbox.next(HOME).unwrap(), first, "not sent yet: still waiting, the same proposal");
		outbox.sent(&first);
		assert_eq!(outbox.next(HOME), None);
		outbox.request(|patch| patch.spin = Some(false));
		assert_eq!(outbox.next(HOME).unwrap().seq, 2);
	}

	#[test]
	fn client_zero_is_the_servers_and_nobody_can_be_it() {
		assert_eq!(Outbox::new(0).next_seq, 1);
		let mut outbox = Outbox::new(0);
		outbox.request(|patch| patch.spin = Some(true));
		assert_eq!(outbox.next(HOME).unwrap().client, 1);
	}

	#[test]
	fn news_from_others_and_from_the_turntable_is_always_believed() {
		let mut outbox = Outbox::new(7);
		outbox.request(|patch| patch.light = Some(1.0));
		outbox.camera_moved();
		let patch = outbox.next(HOME).unwrap();
		outbox.sent(&patch);
		outbox.camera_moved(); // something is waiting again
		assert!(outbox.believe(&scene(8, 99)), "another viewer's change");
		assert!(outbox.believe(&scene(0, 0)), "the server's turntable");
	}

	#[test]
	fn the_echo_of_an_edit_that_has_been_superseded_is_not_believed() {
		let mut outbox = Outbox::new(7);
		for _ in 0..3 {
			outbox.camera_moved();
			let patch = outbox.next(HOME).unwrap();
			outbox.sent(&patch);
		}
		assert!(!outbox.believe(&scene(7, 1)), "an old echo would pull the camera back");
		assert!(!outbox.believe(&scene(7, 2)));
		assert!(outbox.believe(&scene(7, 3)), "the newest echo is the truth: it settles races");
	}

	#[test]
	fn while_something_waits_even_the_newest_echo_is_not_believed() {
		let mut outbox = Outbox::new(7);
		outbox.camera_moved();
		let patch = outbox.next(HOME).unwrap();
		outbox.sent(&patch);
		assert!(outbox.believe(&scene(7, 1)));
		outbox.camera_moved(); // dragged on since
		assert!(!outbox.believe(&scene(7, 1)), "it would undo the part of the drag that was not sent yet");
		outbox.request(|patch| patch.wireframe = Some(true));
		assert!(outbox.is_waiting());
	}

	#[test]
	fn a_new_connection_forgets_what_was_said_before_and_believes_the_server() {
		let mut outbox = Outbox::new(7);
		outbox.camera_moved();
		let patch = outbox.next(HOME).unwrap();
		outbox.sent(&patch);
		outbox.request(|patch| patch.spin = Some(true));
		outbox.reconnected();
		assert!(!outbox.is_waiting(), "the pending change was for a connection that is gone");
		assert!(outbox.believe(&scene(7, 1)), "the server's scene, though it was this page that made it");
		assert_eq!(outbox.next(HOME), None);
		outbox.request(|patch| patch.spin = Some(true));
		assert_eq!(outbox.next(HOME).unwrap().seq, 2, "the counter keeps going");
	}
}
