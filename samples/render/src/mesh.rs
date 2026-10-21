//! Procedural meshes: indexed triangles with per-vertex normals, counter-clockwise when seen from
//! outside, scaled so that every mesh fits in a sphere of the same radius around the origin.

use std::f32::consts::{PI, TAU};

use crate::math::{Vec3, vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
	pub position: Vec3,
	pub normal: Vec3,
}

#[derive(Debug, Clone)]
pub struct Mesh {
	pub vertices: Vec<Vertex>,
	pub triangles: Vec<[u32; 3]>,
	/// Distance from the origin of the farthest vertex.
	pub radius: f32,
}

/// What every mesh is scaled to, so the camera distances mean the same for all of them.
pub const FIT_RADIUS: f32 = 1.4;

impl Mesh {
	pub fn new(vertices: Vec<Vertex>, triangles: Vec<[u32; 3]>) -> Mesh {
		let radius = vertices.iter().map(|vertex| vertex.position.length()).fold(0.0, f32::max);
		Mesh { vertices, triangles, radius }
	}

	fn fitted(mut self) -> Mesh {
		if self.radius > 0.0 {
			let scale = FIT_RADIUS / self.radius;
			for vertex in &mut self.vertices {
				vertex.position = vertex.position * scale;
			}
			self.radius = FIT_RADIUS;
		}
		self
	}

	/// A (2,3) torus knot: a tube following a curve that winds twice around one axis and three times
	/// around the tube of a torus. `segments` along the curve, `sides` around the tube.
	pub fn torus_knot(segments: u32, sides: u32) -> Mesh {
		let (p, q, tube) = (2.0_f32, 3.0_f32, 0.32_f32);
		let curve = |u: f32| {
			let radius = 2.0 + (q * u).cos();
			vec3(radius * (p * u).cos(), radius * (p * u).sin(), -(q * u).sin()) * 0.5
		};
		let mut vertices = Vec::with_capacity((segments * sides) as usize);
		for segment in 0..segments {
			let u = TAU * segment as f32 / segments as f32;
			// a frame along the curve from finite differences: tangent and the direction it bends
			let step = 1e-3;
			let (before, here, after) = (curve(u - step), curve(u), curve(u + step));
			let tangent = (after - before).normalized();
			let bend = after + before - here * 2.0;
			let binormal = tangent.cross(bend).normalized();
			let normal = binormal.cross(tangent);
			for side in 0..sides {
				let v = TAU * side as f32 / sides as f32;
				let outward = normal * v.cos() + binormal * v.sin();
				vertices.push(Vertex { position: here + outward * tube, normal: outward });
			}
		}
		// the frame along this curve is left-handed, hence the flipped winding
		Mesh::new(vertices, grid_triangles(segments, sides).into_iter().map(|[a, b, c]| [a, c, b]).collect())
			.fitted()
	}

	/// A torus around the vertical axis.
	pub fn torus(segments: u32, sides: u32) -> Mesh {
		let (major, minor) = (1.0_f32, 0.42_f32);
		let mut vertices = Vec::with_capacity((segments * sides) as usize);
		for segment in 0..segments {
			let u = TAU * segment as f32 / segments as f32;
			let centre = vec3(major * u.cos(), 0.0, major * u.sin());
			let radial = vec3(u.cos(), 0.0, u.sin());
			for side in 0..sides {
				let v = TAU * side as f32 / sides as f32;
				let outward = radial * v.cos() + vec3(0.0, v.sin(), 0.0);
				vertices.push(Vertex { position: centre + outward * minor, normal: outward });
			}
		}
		// the surface runs the other way round than the knot's, hence the flipped winding
		Mesh::new(vertices, grid_triangles(segments, sides).into_iter().map(|[a, b, c]| [a, c, b]).collect())
			.fitted()
	}

	/// A UV sphere: `rings` from pole to pole, `sectors` around.
	pub fn sphere(rings: u32, sectors: u32) -> Mesh {
		let mut vertices = Vec::new();
		for ring in 0..=rings {
			let polar = PI * ring as f32 / rings as f32;
			for sector in 0..=sectors {
				let azimuth = TAU * sector as f32 / sectors as f32;
				let normal = vec3(polar.sin() * azimuth.cos(), polar.cos(), polar.sin() * azimuth.sin());
				vertices.push(Vertex { position: normal, normal });
			}
		}
		let mut triangles = Vec::new();
		let row = sectors + 1;
		for ring in 0..rings {
			for sector in 0..sectors {
				let (a, b) = (ring * row + sector, (ring + 1) * row + sector);
				if ring != 0 {
					triangles.push([a, a + 1, b]);
				}
				if ring != rings - 1 {
					triangles.push([a + 1, b + 1, b]);
				}
			}
		}
		Mesh::new(vertices, triangles).fitted()
	}

	/// A faceted gem: an icosahedron with every face split in four and pushed out unevenly, lit flat.
	pub fn gem() -> Mesh {
		let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
		let corners: Vec<Vec3> = [
			(-1.0, golden, 0.0), (1.0, golden, 0.0), (-1.0, -golden, 0.0), (1.0, -golden, 0.0),
			(0.0, -1.0, golden), (0.0, 1.0, golden), (0.0, -1.0, -golden), (0.0, 1.0, -golden),
			(golden, 0.0, -1.0), (golden, 0.0, 1.0), (-golden, 0.0, -1.0), (-golden, 0.0, 1.0),
		]
		.into_iter()
		.map(|(x, y, z)| vec3(x, y, z).normalized())
		.collect();
		let faces: [[usize; 3]; 20] = [
			[0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
			[1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
			[3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
			[4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
		];
		let mut vertices = Vec::new();
		let mut triangles = Vec::new();
		for [a, b, c] in faces {
			let (a, b, c) = (corners[a], corners[b], corners[c]);
			let (ab, bc, ca) = ((a + b).normalized(), (b + c).normalized(), (c + a).normalized());
			// a deterministic "cut": every vertex is pushed out by an amount that depends only on where
			// it is, so the corners that neighbouring faces share stay together and the gem stays closed
			let push = |p: Vec3| p * (1.0 + 0.16 * (p.x * 7.0 + p.y * 13.0 + p.z * 3.0).sin());
			for [p, q, r] in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
				let (p, q, r) = (push(p), push(q), push(r));
				let normal = (q - p).cross(r - p).normalized();
				let base = vertices.len() as u32;
				for position in [p, q, r] {
					vertices.push(Vertex { position, normal });
				}
				triangles.push([base, base + 1, base + 2]);
			}
		}
		Mesh::new(vertices, triangles).fitted()
	}
}

/// Triangles for a `segments` x `sides` grid of vertices that wraps in both directions.
fn grid_triangles(segments: u32, sides: u32) -> Vec<[u32; 3]> {
	let mut triangles = Vec::with_capacity((segments * sides * 2) as usize);
	for segment in 0..segments {
		let next = (segment + 1) % segments;
		for side in 0..sides {
			let following = (side + 1) % sides;
			let (a, b) = (segment * sides + side, next * sides + side);
			let (c, d) = (next * sides + following, segment * sides + following);
			triangles.push([a, b, c]);
			triangles.push([a, c, d]);
		}
	}
	triangles
}
