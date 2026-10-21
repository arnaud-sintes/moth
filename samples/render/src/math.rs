//! Just enough vector maths for an orbit camera and shading.

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

pub const fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
	Vec3 { x, y, z }
}

impl Vec3 {
	pub fn dot(self, other: Vec3) -> f32 {
		self.x * other.x + self.y * other.y + self.z * other.z
	}

	pub fn cross(self, other: Vec3) -> Vec3 {
		vec3(
			self.y * other.z - self.z * other.y,
			self.z * other.x - self.x * other.z,
			self.x * other.y - self.y * other.x,
		)
	}

	pub fn length(self) -> f32 {
		self.dot(self).sqrt()
	}

	/// Unit length; the zero vector stays zero instead of becoming NaN.
	pub fn normalized(self) -> Vec3 {
		let length = self.length();
		if length > 1e-12 { self * (1.0 / length) } else { self }
	}
}

impl Add for Vec3 {
	type Output = Vec3;
	fn add(self, other: Vec3) -> Vec3 {
		vec3(self.x + other.x, self.y + other.y, self.z + other.z)
	}
}

impl Sub for Vec3 {
	type Output = Vec3;
	fn sub(self, other: Vec3) -> Vec3 {
		vec3(self.x - other.x, self.y - other.y, self.z - other.z)
	}
}

impl Mul<f32> for Vec3 {
	type Output = Vec3;
	fn mul(self, factor: f32) -> Vec3 {
		vec3(self.x * factor, self.y * factor, self.z * factor)
	}
}

impl Neg for Vec3 {
	type Output = Vec3;
	fn neg(self) -> Vec3 {
		vec3(-self.x, -self.y, -self.z)
	}
}

/// A right-handed camera looking at the origin from a point on a sphere around it.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
	pub eye: Vec3,
	right: Vec3,
	up: Vec3,
	forward: Vec3,
}

impl Camera {
	/// `yaw` turns around the vertical axis (0 looks from +z), `pitch` raises the eye above the
	/// horizon, `distance` is from the origin.
	pub fn orbit(yaw: f32, pitch: f32, distance: f32) -> Camera {
		let eye = vec3(
			distance * pitch.cos() * yaw.sin(),
			distance * pitch.sin(),
			distance * pitch.cos() * yaw.cos(),
		);
		let forward = (-eye).normalized();
		let right = forward.cross(vec3(0.0, 1.0, 0.0)).normalized();
		let up = right.cross(forward);
		Camera { eye, right, up, forward }
	}

	/// A world-space point in camera space: x right, y up, z towards the viewer.
	pub fn to_view(&self, point: Vec3) -> Vec3 {
		let relative = point - self.eye;
		vec3(relative.dot(self.right), relative.dot(self.up), -relative.dot(self.forward))
	}

	/// The same for a direction (no translation).
	pub fn direction_to_view(&self, direction: Vec3) -> Vec3 {
		vec3(direction.dot(self.right), direction.dot(self.up), -direction.dot(self.forward))
	}
}
