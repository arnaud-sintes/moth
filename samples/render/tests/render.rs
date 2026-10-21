use std::collections::HashMap;

use moth_sample_render::{
	Mesh, Settings,
	math::{Vec3, vec3},
	mesh::{FIT_RADIUS, Vertex},
	render,
};

fn meshes() -> Vec<(&'static str, Mesh)> {
	vec![
		("torus knot", Mesh::torus_knot(240, 16)),
		("torus", Mesh::torus(64, 32)),
		("sphere", Mesh::sphere(24, 48)),
		("gem", Mesh::gem()),
	]
}

fn key(point: Vec3) -> (i32, i32, i32) {
	let q = |v: f32| (v * 1e4).round() as i32;
	(q(point.x), q(point.y), q(point.z))
}

// ---- meshes

#[test]
fn indices_are_in_range_and_numbers_are_finite() {
	for (name, mesh) in meshes() {
		let count = mesh.vertices.len() as u32;
		assert!(!mesh.triangles.is_empty(), "{name}");
		for triangle in &mesh.triangles {
			assert!(triangle.iter().all(|index| *index < count), "{name}: index out of range");
		}
		for vertex in &mesh.vertices {
			let numbers = [vertex.position.x, vertex.position.y, vertex.position.z, vertex.normal.x, vertex.normal.y, vertex.normal.z];
			assert!(numbers.iter().all(|n| n.is_finite()), "{name}");
		}
	}
}

#[test]
fn normals_are_unit_and_every_mesh_is_fitted_to_the_same_radius() {
	for (name, mesh) in meshes() {
		for vertex in &mesh.vertices {
			assert!((vertex.normal.length() - 1.0).abs() < 1e-3, "{name}: normal {:?}", vertex.normal);
		}
		assert!((mesh.radius - FIT_RADIUS).abs() < 1e-4, "{name}: radius {}", mesh.radius);
		let farthest = mesh.vertices.iter().map(|v| v.position.length()).fold(0.0, f32::max);
		assert!((farthest - FIT_RADIUS).abs() < 1e-4, "{name}");
	}
}

#[test]
fn triangles_wind_counter_clockwise_seen_from_outside() {
	// the face normal from the winding must agree with the surface normal the vertices carry
	let mut problems = Vec::new();
	for (name, mesh) in meshes() {
		let (mut checked, mut inwards) = (0, 0);
		for &[a, b, c] in &mesh.triangles {
			let (va, vb, vc) = (mesh.vertices[a as usize], mesh.vertices[b as usize], mesh.vertices[c as usize]);
			let face = (vb.position - va.position).cross(vc.position - va.position);
			if face.length() < 1e-7 {
				continue; // a sliver at a pole: no direction to check
			}
			let surface = va.normal + vb.normal + vc.normal;
			checked += 1;
			if face.normalized().dot(surface.normalized()) <= 0.0 {
				inwards += 1;
			}
		}
		if inwards > 0 || checked <= mesh.triangles.len() * 9 / 10 {
			problems.push(format!("{name}: {inwards} of {checked} checkable triangles face inwards"));
		}
	}
	assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn every_mesh_is_watertight() {
	// each undirected edge, by position (seams repeat vertices), belongs to exactly two triangles
	for (name, mesh) in meshes() {
		let mut edges: HashMap<((i32, i32, i32), (i32, i32, i32)), u32> = HashMap::new();
		for &[a, b, c] in &mesh.triangles {
			let points = [a, b, c].map(|i| key(mesh.vertices[i as usize].position));
			for (p, q) in [(points[0], points[1]), (points[1], points[2]), (points[2], points[0])] {
				if p == q {
					continue;
				}
				*edges.entry(if p < q { (p, q) } else { (q, p) }).or_default() += 1;
			}
		}
		let bad = edges.values().filter(|count| **count != 2).count();
		assert_eq!(bad, 0, "{name}: {bad} of {} edges are not shared by exactly two triangles", edges.len());
	}
}

#[test]
fn the_meshes_have_the_expected_size() {
	assert_eq!(Mesh::torus_knot(240, 16).triangles.len(), 240 * 16 * 2);
	assert_eq!(Mesh::torus(64, 32).triangles.len(), 64 * 32 * 2);
	assert_eq!(Mesh::gem().triangles.len(), 20 * 4);
}

// ---- rendering

fn settings() -> Settings {
	Settings { width: 320, height: 240, ..Settings::default() }
}

fn background_at(image: &moth_sample_render::Image, x: u32, y: u32) -> bool {
	let [r, g, b] = image.pixel(x, y);
	u32::from(r) + u32::from(g) + u32::from(b) < 180
}

/// Share of the pixels that are not backdrop (the backdrop is dark and blue).
fn coverage(image: &moth_sample_render::Image) -> f32 {
	let covered = (0..image.height)
		.flat_map(|y| (0..image.width).map(move |x| (x, y)))
		.filter(|(x, y)| !background_at(image, *x, *y))
		.count();
	covered as f32 / (image.width * image.height) as f32
}

#[test]
fn the_image_has_the_requested_size() {
	let image = render(&Mesh::sphere(12, 24), &Settings { width: 100, height: 60, ..settings() });
	assert_eq!((image.width, image.height), (100, 60));
	assert_eq!(image.rgb.len(), 100 * 60 * 3);
	let supersampled = render(&Mesh::sphere(12, 24), &Settings { width: 100, height: 60, supersample: 3, ..settings() });
	assert_eq!((supersampled.width, supersampled.height), (100, 60));
}

#[test]
fn rendering_is_deterministic() {
	let mesh = Mesh::torus_knot(120, 12);
	let settings = Settings { supersample: 2, wireframe: true, ..settings() };
	assert_eq!(render(&mesh, &settings), render(&mesh, &settings));
}

#[test]
fn a_sphere_covers_the_area_projection_predicts() {
	// radius 1.4 seen from 5 units with a 38 degree field of view: a circle of about 0.85 of the
	// half-height. Any error in the projection, the culling or the winding moves this a lot.
	let image = render(&Mesh::sphere(48, 96), &Settings { width: 480, height: 480, supersample: 2, ..settings() });
	let radius = 0.5 * 480.0 * (1.4_f32 / (25.0_f32 - 1.96).sqrt()) / (19.0_f32.to_radians().tan());
	let expected = std::f32::consts::PI * radius * radius / (480.0 * 480.0);
	let measured = coverage(&image);
	assert!((measured - expected).abs() < 0.02, "covers {measured:.3}, expected {expected:.3}");
	assert!(!background_at(&image, 240, 240) && background_at(&image, 2, 2));
}

#[test]
fn a_farther_camera_makes_the_object_smaller() {
	let mesh = Mesh::sphere(32, 64);
	let near = coverage(&render(&mesh, &Settings { distance: 4.0, ..settings() }));
	let far = coverage(&render(&mesh, &Settings { distance: 8.0, ..settings() }));
	assert!(far < near * 0.45 && far > near * 0.15, "near {near:.3}, far {far:.3}");
}

#[test]
fn turning_the_camera_changes_the_picture() {
	let mesh = Mesh::torus_knot(160, 12);
	let before = render(&mesh, &settings());
	let after = render(&mesh, &Settings { yaw: 1.4, ..settings() });
	let differing = before.rgb.iter().zip(&after.rgb).filter(|(a, b)| a != b).count();
	assert!(differing > before.rgb.len() / 10);
}

#[test]
fn the_light_and_the_material_change_the_shading_but_not_the_shape() {
	let mesh = Mesh::sphere(32, 64);
	let lit_left = render(&mesh, &Settings { light_azimuth: -1.2, ..settings() });
	let lit_right = render(&mesh, &Settings { light_azimuth: 1.2, ..settings() });
	let (left_of_centre, right_of_centre) = ((120, 120), (200, 120));
	let brightness = |image: &moth_sample_render::Image, at: (u32, u32)| image.pixel(at.0, at.1).iter().map(|c| u32::from(*c)).sum::<u32>();
	assert!(brightness(&lit_left, left_of_centre) > brightness(&lit_left, right_of_centre));
	assert!(brightness(&lit_right, right_of_centre) > brightness(&lit_right, left_of_centre));
	assert!((coverage(&lit_left) - coverage(&lit_right)).abs() < 0.01);

	let jade = render(&mesh, &Settings { material: moth_sample_render::Material::JADE, ..settings() });
	let ruby = render(&mesh, &Settings { material: moth_sample_render::Material::RUBY, ..settings() });
	let [jr, jg, _] = jade.pixel(160, 130);
	let [rr, rg, _] = ruby.pixel(160, 130);
	assert!(jg > jr && rr > rg, "jade is greener, ruby redder");
}

#[test]
fn the_wireframe_draws_the_edges_on_top() {
	let mesh = Mesh::torus(32, 16);
	let plain = render(&mesh, &settings());
	let wired = render(&mesh, &Settings { wireframe: true, ..settings() });
	let differing = plain.rgb.iter().zip(&wired.rgb).filter(|(a, b)| a != b).count();
	assert!(differing > 500, "{differing}");
	assert!((coverage(&plain) - coverage(&wired)).abs() < 0.005, "the silhouette is the same");
	// wire pixels are light and cyan: more blue and green than red
	let cyan = (0..wired.width * wired.height)
		.filter(|i| {
			let [r, g, b] = wired.pixel(i % wired.width, i / wired.width);
			g > 200 && b > 200 && r < g
		})
		.count();
	assert!(cyan > 100, "{cyan}");
}

#[test]
fn supersampling_smooths_the_edge_without_moving_it() {
	let mesh = Mesh::sphere(48, 96);
	let aliased = render(&mesh, &Settings { supersample: 1, ..settings() });
	let smooth = render(&mesh, &Settings { supersample: 3, ..settings() });
	assert!((coverage(&aliased) - coverage(&smooth)).abs() < 0.01);
	// along the silhouette the smooth image has intermediate values the aliased one lacks
	let distinct = |image: &moth_sample_render::Image| {
		let mut values: Vec<[u8; 3]> = (0..image.width).map(|x| image.pixel(x, 120)).collect();
		values.sort_unstable();
		values.dedup();
		values.len()
	};
	assert!(distinct(&smooth) > distinct(&aliased) - 10);
}

// ---- the rasterizer's rules, with hand-made meshes

fn triangle(corners: [Vec3; 3], normal: Vec3) -> Mesh {
	let vertices = corners.map(|position| Vertex { position, normal }).to_vec();
	Mesh::new(vertices, vec![[0, 1, 2]])
}

fn facing() -> Settings {
	// looking straight at the z = 0 plane from +z
	Settings { yaw: 0.0, pitch: 0.0, distance: 4.0, light_azimuth: 0.0, ..settings() }
}

const CCW_FROM_PLUS_Z: [Vec3; 3] = [vec3(-1.5, -1.5, 0.0), vec3(1.5, -1.5, 0.0), vec3(0.0, 1.5, 0.0)];

#[test]
fn a_triangle_is_seen_from_its_front_and_culled_from_behind() {
	let front = render(&triangle(CCW_FROM_PLUS_Z, vec3(0.0, 0.0, 1.0)), &facing());
	assert!(!background_at(&front, 160, 130), "counter-clockwise from the camera: drawn");

	let [a, b, c] = CCW_FROM_PLUS_Z;
	let back = render(&triangle([a, c, b], vec3(0.0, 0.0, 1.0)), &facing());
	assert!(background_at(&back, 160, 130), "clockwise from the camera: culled");
}

#[test]
fn the_nearer_surface_wins_whatever_the_drawing_order() {
	let near = [vec3(-1.5, -1.5, 0.5), vec3(1.5, -1.5, 0.5), vec3(0.0, 1.5, 0.5)];
	let far = [vec3(-1.5, -1.5, -0.5), vec3(1.5, -1.5, -0.5), vec3(0.0, 1.5, -0.5)];
	// told apart by their normals: the near one faces the light, the far one faces away from it
	let both = |near_first: bool| {
		let mut vertices = Vec::new();
		let mut triangles = Vec::new();
		let mut add = |corners: [Vec3; 3], normal: Vec3| {
			let base = vertices.len() as u32;
			vertices.extend(corners.map(|position| Vertex { position, normal }));
			triangles.push([base, base + 1, base + 2]);
		};
		if near_first {
			add(near, vec3(0.0, 0.0, 1.0));
			add(far, vec3(0.0, 0.0, -1.0));
		} else {
			add(far, vec3(0.0, 0.0, -1.0));
			add(near, vec3(0.0, 0.0, 1.0));
		}
		render(&Mesh::new(vertices, triangles), &facing())
	};
	let alone = render(&triangle(near, vec3(0.0, 0.0, 1.0)), &facing());
	let far_alone = render(&triangle(far, vec3(0.0, 0.0, -1.0)), &facing());
	assert_ne!(alone.pixel(160, 130), far_alone.pixel(160, 130), "the two must be tellable apart");
	assert_eq!(both(true).pixel(160, 130), alone.pixel(160, 130));
	assert_eq!(both(false).pixel(160, 130), alone.pixel(160, 130));
}

#[test]
fn geometry_off_the_screen_or_around_the_camera_is_harmless() {
	// far off to the side, huge, and the camera inside the mesh
	let huge = [vec3(-1e6, -1e6, 0.0), vec3(1e6, -1e6, 0.0), vec3(0.0, 1e6, 0.0)];
	let image = render(&triangle(huge, vec3(0.0, 0.0, 1.0)), &facing());
	assert!(!background_at(&image, 160, 120), "a triangle that covers the screen covers it");
	let aside = [vec3(50.0, 50.0, 0.0), vec3(52.0, 50.0, 0.0), vec3(51.0, 52.0, 0.0)];
	render(&triangle(aside, vec3(0.0, 0.0, 1.0)), &facing());
	render(&Mesh::sphere(8, 16), &Settings { distance: 0.1, ..settings() });
	render(&Mesh::sphere(8, 16), &Settings { distance: 0.0, ..settings() });
}

#[test]
fn degenerate_sizes_do_not_panic() {
	let mesh = Mesh::gem();
	for (width, height, samples) in [(1, 1, 1), (1, 1, 4), (3, 2, 4), (5, 400, 2), (400, 5, 2), (0, 0, 0)] {
		let image = render(&mesh, &Settings { width, height, supersample: samples, ..settings() });
		assert_eq!(image.rgb.len() as u32, image.width * image.height * 3);
	}
}

// ---- reducing a picture

fn flat(width: u32, height: u32, colour: [u8; 3]) -> moth_sample_render::Image {
	moth_sample_render::Image { width, height, rgb: colour.iter().copied().cycle().take((width * height * 3) as usize).collect() }
}

#[test]
fn resizing_to_the_same_size_changes_nothing() {
	let image = render(&Mesh::torus(16, 8), &Settings { width: 64, height: 48, ..settings() });
	assert_eq!(image.resized(64, 48), image);
}

#[test]
fn a_flat_colour_stays_exactly_itself_at_any_size() {
	for (from, to) in [((100, 60), (37, 22)), ((100, 60), (99, 59)), ((8, 8), (3, 5)), ((5, 5), (1, 1)), ((16, 9), (160, 90))] {
		let image = flat(from.0, from.1, [201, 17, 99]).resized(to.0, to.1);
		assert_eq!((image.width, image.height), to);
		assert!(image.rgb.chunks(3).all(|pixel| pixel == [201, 17, 99]), "{from:?} -> {to:?}");
	}
}

#[test]
fn each_new_pixel_is_the_average_of_what_it_covers() {
	// a 2x2 checkerboard of black and white reduced to one grey pixel
	let checks = moth_sample_render::Image { width: 2, height: 2, rgb: vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0] };
	assert_eq!(checks.resized(1, 1).rgb, vec![128, 128, 128]);

	// halving a gradient averages neighbours: 0, 100, 200, 40 -> 50, 120
	let row = moth_sample_render::Image { width: 4, height: 1, rgb: [0, 100, 200, 40].iter().flat_map(|v| [*v, *v, *v]).collect() };
	let half = row.resized(2, 1);
	assert_eq!((half.pixel(0, 0), half.pixel(1, 0)), ([50, 50, 50], [120, 120, 120]));

	// a width that is not a multiple weighs the pixel in the middle in halves: 3 -> 2 is (0, 90/2) and (90/2, 180)
	let three = moth_sample_render::Image { width: 3, height: 1, rgb: [0, 90, 180].iter().flat_map(|v| [*v, *v, *v]).collect() };
	let two = three.resized(2, 1);
	assert_eq!((two.pixel(0, 0), two.pixel(1, 0)), ([30, 30, 30], [150, 150, 150]));
}

#[test]
fn reducing_keeps_the_shape_and_the_brightness_of_a_render() {
	let image = render(&Mesh::torus_knot(80, 8), &Settings { width: 320, height: 200, ..settings() });
	let small = image.resized(160, 100);
	assert_eq!((small.width, small.height), (160, 100));
	let mean = |image: &moth_sample_render::Image| image.rgb.iter().map(|v| f64::from(*v)).sum::<f64>() / image.rgb.len() as f64;
	assert!((mean(&image) - mean(&small)).abs() < 1.0, "{} vs {}", mean(&image), mean(&small));
	assert!((coverage(&image) - coverage(&small)).abs() < 0.01, "the object covers the same share of the picture");
}

#[test]
fn degenerate_resizes_do_not_panic() {
	let image = render(&Mesh::gem(), &Settings { width: 40, height: 30, ..settings() });
	for (width, height) in [(0, 0), (1, 1), (1, 30), (40, 1), (400, 300), (41, 31)] {
		let resized = image.resized(width, height);
		assert_eq!(resized.rgb.len() as u32, resized.width * resized.height * 3);
	}
}

// ---- the shape of the picture

/// The box around what is not backdrop: (left, top, right, bottom), inclusive.
fn object_box(image: &moth_sample_render::Image) -> (u32, u32, u32, u32) {
	let (mut left, mut top, mut right, mut bottom) = (image.width, image.height, 0, 0);
	for y in 0..image.height {
		for x in 0..image.width {
			if !background_at(image, x, y) {
				(left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
			}
		}
	}
	(left, top, right, bottom)
}

#[test]
fn a_sphere_is_round_and_fits_whatever_the_shape_of_the_picture() {
	let sphere = Mesh::sphere(48, 96);
	let mut diameters = Vec::new();
	for (width, height) in [(240, 240), (240, 600), (240, 1000), (600, 240), (1000, 240)] {
		let image = render(&sphere, &Settings { width, height, distance: 6.3, ..settings() });
		let (left, top, right, bottom) = object_box(&image);
		let (across, down) = (right - left + 1, bottom - top + 1);
		assert!(left > 0 && top > 0 && right < width - 1 && bottom < height - 1, "{width}x{height}: the sphere is cut off by the picture ({left},{top})-({right},{bottom})");
		assert!(across.abs_diff(down) <= 2, "{width}x{height}: {across} wide and {down} tall is not round");
		diameters.push(across);
	}
	// the field of view is that of the shorter side: the sphere is as big as it is in a square picture of
	// that side, however much longer the other side is
	assert!(diameters.iter().all(|d| d.abs_diff(diameters[0]) <= 2), "{diameters:?}");
}
