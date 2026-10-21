//! The rasterizer. Everything is computed in camera space (x right, y up, z towards the viewer,
//! the camera at the origin), so the key light is fixed relative to the camera and the object turns
//! under it. Pixels are shaded once per covered sample (perspective-correct interpolation of the
//! normal and position), then supersamples are averaged. The image is cut in horizontal bands that
//! are rasterized on separate threads.

use std::thread;

use crate::{
	math::{Camera, Vec3, vec3},
	mesh::Mesh,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
	/// Linear RGB, each 0..=1.
	pub base: [f32; 3],
	/// How strong the highlight is, 0..=1.
	pub specular: f32,
}

impl Material {
	pub const BRONZE: Material = Material { base: [0.80, 0.50, 0.20], specular: 0.9 };
	pub const JADE: Material = Material { base: [0.18, 0.62, 0.42], specular: 0.6 };
	pub const ICE: Material = Material { base: [0.55, 0.78, 0.95], specular: 1.0 };
	pub const RUBY: Material = Material { base: [0.78, 0.07, 0.16], specular: 0.8 };
	pub const CLAY: Material = Material { base: [0.72, 0.68, 0.62], specular: 0.1 };
}

#[derive(Debug, Clone, Copy)]
pub struct Settings {
	pub width: u32,
	pub height: u32,
	/// Camera, see [`Camera::orbit`].
	pub yaw: f32,
	pub pitch: f32,
	pub distance: f32,
	/// Where the key light is, in radians around the camera's vertical axis (0: from the viewer).
	pub light_azimuth: f32,
	/// Highlight tightness (the Blinn-Phong exponent).
	pub shininess: f32,
	pub wireframe: bool,
	/// Samples per pixel along each axis, 1 to 4.
	pub supersample: u32,
	pub material: Material,
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			width: 640,
			height: 480,
			yaw: 0.6,
			pitch: 0.35,
			distance: 5.0,
			light_azimuth: 0.7,
			shininess: 48.0,
			wireframe: false,
			supersample: 1,
			material: Material::BRONZE,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
	pub width: u32,
	pub height: u32,
	/// `width * height` pixels, 3 bytes each, row by row from the top.
	pub rgb: Vec<u8>,
}

impl Image {
	pub fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
		let at = ((y * self.width + x) * 3) as usize;
		[self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]]
	}

	/// This picture at another size (at least 1x1): each new pixel is the average of the part of the
	/// picture it covers (a box filter), so reducing is smooth and anti-aliased, and a flat colour stays
	/// exactly itself. The aspect ratio is the caller's business.
	pub fn resized(&self, width: u32, height: u32) -> Image {
		let (width, height) = (width.max(1), height.max(1));
		if (width, height) == (self.width, self.height) || self.width == 0 || self.height == 0 {
			return self.clone();
		}
		let across = axis_weights(self.width, width);
		let down = axis_weights(self.height, height);
		let (source_width, source_height) = (self.width as usize, self.height as usize);

		// first along the rows, then along the columns
		let mut wide = vec![0.0_f32; width as usize * source_height * 3];
		for y in 0..source_height {
			let row = &self.rgb[y * source_width * 3..(y + 1) * source_width * 3];
			for (x, weights) in across.iter().enumerate() {
				let mut sum = [0.0_f32; 3];
				for &(from, weight) in weights {
					for channel in 0..3 {
						sum[channel] += f32::from(row[from * 3 + channel]) * weight;
					}
				}
				wide[(y * width as usize + x) * 3..][..3].copy_from_slice(&sum);
			}
		}
		let mut rgb = vec![0_u8; width as usize * height as usize * 3];
		for (y, weights) in down.iter().enumerate() {
			for x in 0..width as usize {
				let mut sum = [0.0_f32; 3];
				for &(from, weight) in weights {
					for channel in 0..3 {
						sum[channel] += wide[(from * width as usize + x) * 3 + channel] * weight;
					}
				}
				for channel in 0..3 {
					rgb[(y * width as usize + x) * 3 + channel] = sum[channel].round().clamp(0.0, 255.0) as u8;
				}
			}
		}
		Image { width, height, rgb }
	}
}

/// For each of `to` new pixels along an axis, the source pixels (of `from`) it covers and how much of
/// the new pixel each one is: the weights of a new pixel add up to 1.
fn axis_weights(from: u32, to: u32) -> Vec<Vec<(usize, f32)>> {
	let step = f64::from(from) / f64::from(to);
	(0..to)
		.map(|index| {
			let (start, end) = (f64::from(index) * step, f64::from(index + 1) * step);
			let first = start.floor() as usize;
			let last = (end.ceil() as usize).min(from as usize);
			(first..last)
				.filter_map(|pixel| {
					let overlap = end.min((pixel + 1) as f64) - start.max(pixel as f64);
					(overlap > 0.0).then(|| (pixel, (overlap / step) as f32))
				})
				.collect()
		})
		.collect()
}

const FIELD_OF_VIEW: f32 = 38.0 * std::f32::consts::PI / 180.0;
const NEAR: f32 = 0.05;

/// A vertex in camera space and on the screen (in samples).
#[derive(Clone, Copy)]
struct Projected {
	x: f32,
	y: f32,
	/// 1 / distance along the view axis; larger is nearer, and it is linear on the screen.
	inverse_w: f32,
	position: Vec3,
	normal: Vec3,
}

/// A triangle that faces the camera, ready to be rasterized.
struct Setup {
	vertices: [Projected; 3],
	min_x: i32,
	max_x: i32,
	min_y: i32,
	max_y: i32,
	/// The (negative, for a front face) double area in samples.
	area: f32,
	/// Distance in samples from each vertex to the opposite edge, for the wireframe.
	altitude: [f32; 3],
}

struct Light {
	key: Vec3,
	fill: Vec3,
	material: Material,
	shininess: f32,
	wire: Option<f32>,
}

fn edge(a: &Projected, b: &Projected, x: f32, y: f32) -> f32 {
	(b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}

pub fn render(mesh: &Mesh, settings: &Settings) -> Image {
	let samples = settings.supersample.clamp(1, 4);
	let (width, height) = (settings.width.max(1) * samples, settings.height.max(1) * samples);
	let camera = Camera::orbit(settings.yaw, settings.pitch, settings.distance);
	let setups = setup(mesh, &camera, width, height);

	let light = Light {
		key: vec3(settings.light_azimuth.sin(), 0.55, settings.light_azimuth.cos()).normalized(),
		fill: vec3(-0.7, -0.15, 0.45).normalized(),
		material: settings.material,
		shininess: settings.shininess.max(1.0),
		wire: settings.wireframe.then_some(0.9 * samples as f32),
	};

	let mut colour = vec![[0_u8; 3]; (width * height) as usize];
	let mut depth = vec![0.0_f32; (width * height) as usize];
	let bands = thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16).min((height as usize / 16).max(1));
	let rows = (height as usize).div_ceil(bands);
	thread::scope(|scope| {
		let slices = colour.chunks_mut(rows * width as usize).zip(depth.chunks_mut(rows * width as usize));
		for (index, (colour, depth)) in slices.enumerate() {
			let (setups, light) = (&setups, &light);
			scope.spawn(move || {
				let first = (index * rows) as i32;
				background(colour, width, height, first);
				for triangle in setups {
					rasterize(triangle, colour, depth, width, first, light);
				}
			});
		}
	});

	downsample(&colour, width, height, samples)
}

/// Project the vertices and keep the triangles that face the camera and are in front of it.
fn setup(mesh: &Mesh, camera: &Camera, width: u32, height: u32) -> Vec<Setup> {
	let aspect = width as f32 / height as f32;
	// the field of view is that of the shorter side, so that the object fits a tall picture across its
	// width just as it fits a wide one across its height
	let focal = (1.0 / (FIELD_OF_VIEW / 2.0).tan()) * aspect.min(1.0);
	let projected: Vec<Option<Projected>> = mesh
		.vertices
		.iter()
		.map(|vertex| {
			let position = camera.to_view(vertex.position);
			let w = -position.z;
			(w > NEAR).then(|| Projected {
				x: (position.x * focal / aspect / w * 0.5 + 0.5) * width as f32,
				y: (0.5 - position.y * focal / w * 0.5) * height as f32,
				inverse_w: 1.0 / w,
				position,
				normal: camera.direction_to_view(vertex.normal).normalized(),
			})
		})
		.collect();

	let mut setups = Vec::with_capacity(mesh.triangles.len() / 2);
	for &[a, b, c] in &mesh.triangles {
		let (Some(a), Some(b), Some(c)) =
			(projected[a as usize], projected[b as usize], projected[c as usize])
		else {
			continue;
		};
		// y grows downwards on the screen, so a face that is seen counter-clockwise has a negative area
		let area = edge(&a, &b, c.x, c.y);
		if area >= -1e-6 {
			continue;
		}
		let (min_x, max_x) = (a.x.min(b.x).min(c.x).floor() as i32, a.x.max(b.x).max(c.x).ceil() as i32);
		let (min_y, max_y) = (a.y.min(b.y).min(c.y).floor() as i32, a.y.max(b.y).max(c.y).ceil() as i32);
		if max_x < 0 || max_y < 0 || min_x >= width as i32 || min_y >= height as i32 {
			continue;
		}
		let length = |p: &Projected, q: &Projected| ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt().max(1e-6);
		let altitude = [
			-area / length(&b, &c),
			-area / length(&c, &a),
			-area / length(&a, &b),
		];
		setups.push(Setup { vertices: [a, b, c], min_x, max_x, min_y, max_y, area, altitude });
	}
	setups
}

/// Fill a band of rows with the backdrop: a vertical gradient with a soft vignette.
fn background(colour: &mut [[u8; 3]], width: u32, height: u32, first_row: i32) {
	for (index, pixel) in colour.iter_mut().enumerate() {
		let (x, y) = ((index as u32 % width) as f32 / width as f32, (first_row as u32 + index as u32 / width) as f32 / height as f32);
		let (dx, dy) = (x - 0.5, y - 0.5);
		let vignette = 1.0 - 0.55 * (dx * dx + dy * dy);
		let top = [16.0, 20.0, 34.0];
		let bottom = [34.0, 42.0, 66.0];
		for channel in 0..3 {
			pixel[channel] = ((top[channel] + (bottom[channel] - top[channel]) * y) * vignette) as u8;
		}
	}
}

/// Rasterize one triangle into a band that starts at row `first`.
fn rasterize(triangle: &Setup, colour: &mut [[u8; 3]], depth: &mut [f32], width: u32, first: i32, light: &Light) {
	let rows = (colour.len() / width as usize) as i32;
	let (y_start, y_end) = (triangle.min_y.max(first), triangle.max_y.min(first + rows - 1));
	if y_start > y_end {
		return;
	}
	let (x_start, x_end) = (triangle.min_x.max(0), triangle.max_x.min(width as i32 - 1));
	let [a, b, c] = &triangle.vertices;
	let inverse_area = 1.0 / triangle.area;

	for y in y_start..=y_end {
		let py = y as f32 + 0.5;
		// the three edge functions at the start of the row, and how they change per sample
		let px = x_start as f32 + 0.5;
		let mut e0 = edge(b, c, px, py);
		let mut e1 = edge(c, a, px, py);
		let mut e2 = edge(a, b, px, py);
		let (d0, d1, d2) = (-(c.y - b.y), -(a.y - c.y), -(b.y - a.y));
		let row = ((y - first) as usize) * width as usize;
		for x in x_start..=x_end {
			let (l0, l1, l2) = (e0 * inverse_area, e1 * inverse_area, e2 * inverse_area);
			(e0, e1, e2) = (e0 + d0, e1 + d1, e2 + d2);
			if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 {
				continue;
			}
			let inverse_w = l0 * a.inverse_w + l1 * b.inverse_w + l2 * c.inverse_w;
			let at = row + x as usize;
			if inverse_w <= depth[at] {
				continue;
			}
			depth[at] = inverse_w;

			// perspective-correct weights: attributes are linear in 1/w, not on the screen
			let (w0, w1, w2) = (l0 * a.inverse_w / inverse_w, l1 * b.inverse_w / inverse_w, l2 * c.inverse_w / inverse_w);
			let normal = a.normal * w0 + b.normal * w1 + c.normal * w2;
			let position = a.position * w0 + b.position * w1 + c.position * w2;
			let mut rgb = shade(normal.normalized(), position, light);
			if let Some(width) = light.wire {
				let distance = (l0 * triangle.altitude[0]).min(l1 * triangle.altitude[1]).min(l2 * triangle.altitude[2]);
				let coverage = (width + 0.5 - distance).clamp(0.0, 1.0) * 0.85;
				let wire = [0.55, 0.95, 1.0];
				for channel in 0..3 {
					rgb[channel] += (wire[channel] - rgb[channel]) * coverage;
				}
			}
			// gamma 2 is close enough to sRGB and a square root is cheap
			colour[at] = [
				(rgb[0].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
				(rgb[1].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
				(rgb[2].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
			];
		}
	}
}

/// Blinn-Phong with a key light, a cool fill, a sky/ground ambient and a rim light. Linear RGB.
fn shade(normal: Vec3, position: Vec3, light: &Light) -> [f32; 3] {
	let view = (-position).normalized();
	let material = &light.material;

	let key = normal.dot(light.key).max(0.0);
	let fill = normal.dot(light.fill).max(0.0);
	let half = (light.key + view).normalized();
	let highlight = normal.dot(half).max(0.0).powf(light.shininess) * material.specular * (key.sqrt());
	let rim = (1.0 - normal.dot(view).max(0.0)).powi(3);
	let sky = 0.5 + 0.5 * normal.y;

	let mut rgb = [0.0; 3];
	let key_colour = [1.0, 0.95, 0.88];
	let fill_colour = [0.35, 0.45, 0.65];
	let ambient = [0.10 + 0.10 * sky, 0.11 + 0.11 * sky, 0.14 + 0.14 * sky];
	let rim_colour = [0.25, 0.45, 0.8];
	for channel in 0..3 {
		let diffuse = ambient[channel] + key * key_colour[channel] * 0.95 + fill * fill_colour[channel] * 0.5;
		rgb[channel] = material.base[channel] * diffuse + highlight * key_colour[channel] + rim * rim_colour[channel] * 0.5;
	}
	rgb
}

/// Average blocks of `samples` x `samples` into one pixel.
fn downsample(colour: &[[u8; 3]], width: u32, height: u32, samples: u32) -> Image {
	let (out_width, out_height) = (width / samples, height / samples);
	let mut rgb = Vec::with_capacity((out_width * out_height * 3) as usize);
	let area = (samples * samples) as u32;
	for y in 0..out_height {
		for x in 0..out_width {
			let mut sum = [0_u32; 3];
			for sy in 0..samples {
				for sx in 0..samples {
					let pixel = colour[((y * samples + sy) * width + x * samples + sx) as usize];
					for channel in 0..3 {
						sum[channel] += u32::from(pixel[channel]);
					}
				}
			}
			rgb.extend(sum.map(|total| (total / area) as u8));
		}
	}
	Image { width: out_width, height: out_height, rgb }
}
