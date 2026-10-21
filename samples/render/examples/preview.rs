//! Renders every mesh into one contact sheet, to look at: `cargo run -p moth-sample-render --example
//! preview -- out.jpg`. Prints how long each render and the JPEG encode took, which is also a quick
//! way to see whether the optimization settings of the workspace profiles are in effect.

use std::time::Instant;

use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use moth_sample_render::{Material, Mesh, Settings, render};

fn main() {
	let path = std::env::args().nth(1).unwrap_or_else(|| "preview.jpg".into());
	let (width, height) = (480_u32, 360_u32);
	let shots = [
		(Mesh::torus_knot(240, 16), Material::BRONZE, false),
		(Mesh::torus(64, 32), Material::ICE, false),
		(Mesh::sphere(32, 64), Material::JADE, true),
		(Mesh::gem(), Material::RUBY, false),
	];

	let mut sheet = vec![0_u8; (width * 2 * height * 2 * 3) as usize];
	for (index, (mesh, material, wireframe)) in shots.iter().enumerate() {
		let settings = Settings { width, height, material: *material, wireframe: *wireframe, supersample: 2, yaw: 0.7 + index as f32 * 0.5, ..Settings::default() };
		let started = Instant::now();
		let image = render(mesh, &settings);
		println!("model {index}: {} triangles, {:?}", mesh.triangles.len(), started.elapsed());
		let (left, top) = ((index as u32 % 2) * width, (index as u32 / 2) * height);
		for y in 0..height {
			let from = (y * width * 3) as usize;
			let to = (((top + y) * width * 2 + left) * 3) as usize;
			sheet[to..to + (width * 3) as usize].copy_from_slice(&image.rgb[from..from + (width * 3) as usize]);
		}
	}

	// the encode alone, on a frame the size of a large viewport
	let big = render(&Mesh::torus_knot(240, 16), &Settings { width: 1030, height: 686, supersample: 2, ..Settings::default() });
	for (label, sampling) in [("4:4:4", SamplingFactor::F_1_1), ("4:2:0", SamplingFactor::F_2_2)] {
		for quality in [90, 80] {
			let started = Instant::now();
			let mut jpeg = Vec::new();
			let mut encoder = Encoder::new(&mut jpeg, quality);
			encoder.set_sampling_factor(sampling);
			encoder.encode(&big.rgb, big.width as u16, big.height as u16, ColorType::Rgb).expect("encode");
			println!("encode {}x{} {label} q{quality}: {:?}, {} bytes", big.width, big.height, started.elapsed(), jpeg.len());
		}
	}

	Encoder::new_file(&path, 92)
		.and_then(|encoder| encoder.encode(&sheet, (width * 2) as u16, (height * 2) as u16, ColorType::Rgb))
		.expect("write the sheet");
	println!("wrote {path}");
}
