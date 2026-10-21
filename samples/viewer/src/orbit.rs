//! The orbit camera the pointer drives: pure arithmetic, no browser. (The turntable is the
//! server's: it turns the camera for everyone.)

use std::f32::consts::TAU;

/// Radians of rotation per pixel dragged.
const DRAG_RATE: f32 = 0.008;
/// Keeps the camera from flipping over the poles.
const MAX_PITCH: f32 = 1.45;
const MIN_DISTANCE: f32 = 3.0;
const MAX_DISTANCE: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
	pub yaw: f32,
	pub pitch: f32,
	pub distance: f32,
}

impl Orbit {
	pub const HOME: Orbit = Orbit { yaw: 0.6, pitch: 0.35, distance: 6.3 };

	/// The pointer moved by `dx`, `dy` pixels with the button down. Dragging right turns the object
	/// to the right (the camera goes left); dragging down tips its top towards you.
	pub fn drag(&mut self, dx: f32, dy: f32) {
		if !(dx.is_finite() && dy.is_finite()) {
			return;
		}
		self.yaw = (self.yaw - dx * DRAG_RATE).rem_euclid(TAU);
		self.pitch = (self.pitch + dy * DRAG_RATE).clamp(-MAX_PITCH, MAX_PITCH);
	}

	/// The wheel turned by `delta` (positive: away from the user, which moves the camera back).
	/// Exponential, so each notch is the same *proportion* closer, wherever the camera is.
	pub fn zoom(&mut self, delta: f32) {
		if !delta.is_finite() {
			return;
		}
		self.distance = (self.distance * (delta * 0.0012).exp()).clamp(MIN_DISTANCE, MAX_DISTANCE);
	}

}

impl Default for Orbit {
	fn default() -> Self {
		Orbit::HOME
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dragging_right_turns_the_object_right_and_down_tips_it_towards_the_viewer() {
		let mut orbit = Orbit::HOME;
		orbit.drag(50.0, 0.0);
		assert!(orbit.yaw < Orbit::HOME.yaw, "the camera moves left as the object turns right");
		let mut orbit = Orbit::HOME;
		orbit.drag(0.0, 50.0);
		assert!(orbit.pitch > Orbit::HOME.pitch);
		assert_eq!(orbit.yaw, Orbit::HOME.yaw, "a vertical drag leaves the yaw alone");
	}

	#[test]
	fn the_pitch_stops_before_the_poles_however_far_it_is_dragged() {
		let mut orbit = Orbit::HOME;
		orbit.drag(0.0, 1e6);
		assert_eq!(orbit.pitch, MAX_PITCH);
		orbit.drag(0.0, -1e7);
		assert_eq!(orbit.pitch, -MAX_PITCH);
	}

	#[test]
	fn the_yaw_wraps_and_stays_in_one_turn() {
		let mut orbit = Orbit::HOME;
		for _ in 0..1000 {
			orbit.drag(-37.0, 0.0);
			assert!((0.0..TAU).contains(&orbit.yaw), "{}", orbit.yaw);
		}
	}

	#[test]
	fn zooming_is_proportional_monotonic_and_bounded() {
		let mut orbit = Orbit::HOME;
		orbit.zoom(120.0);
		assert!(orbit.distance > Orbit::HOME.distance, "a positive wheel moves the camera back");
		let mut near = Orbit { distance: 4.0, ..Orbit::HOME };
		let mut far = Orbit { distance: 8.0, ..Orbit::HOME };
		near.zoom(-100.0);
		far.zoom(-100.0);
		assert!((near.distance / 4.0 - far.distance / 8.0).abs() < 1e-5, "the same proportion at any distance");
		orbit.zoom(1e9);
		assert_eq!(orbit.distance, MAX_DISTANCE);
		orbit.zoom(-1e9);
		assert_eq!(orbit.distance, MIN_DISTANCE);
	}

	#[test]
	fn a_broken_input_device_cannot_poison_the_camera() {
		// the server sanitizes what it receives, but the client should not send nonsense, nor keep it
		let mut orbit = Orbit::HOME;
		orbit.drag(f32::NAN, 3.0);
		orbit.drag(1.0, f32::INFINITY);
		orbit.zoom(f32::NAN);
		orbit.zoom(f32::NEG_INFINITY);
		assert_eq!(orbit, Orbit::HOME, "non-finite input is ignored entirely");
	}
}
