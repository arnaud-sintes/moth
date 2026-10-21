//! Which viewers can be served by one rendering of the scene.
//!
//! Rendering is the expensive part, so it is done as few times as the viewers' screens allow. Each
//! viewer asks for a frame of some size; viewers are served together when their pictures are alike:
//!
//! - the same shape (aspect ratio) is needed, since a picture can be scaled but not reshaped without
//!   distorting it; a viewer of another shape gets a rendering of its own;
//! - among viewers of one shape, the biggest decides the size of the **rendering**;
//! - a viewer at least [`CLOSE_ENOUGH`] as wide as an output is sent that output as it is, and its
//!   browser scales it to its canvas, so near sizes share even the compression;
//! - one at least [`WORTH_REDUCING`] as wide as the rendering is sent a copy **reduced on the server
//!   before it is compressed** (cheap, and the reduced size is shared by viewers near it);
//! - a smaller one is not worth reducing from: a rendering of its own is cheaper than a big one, so it
//!   starts a new group;
//! - a picture is only shared by viewers who want the same JPEG quality: the rendering is shared all
//!   the same, and each quality is compressed on its own.
//!
//! Pure arithmetic over sizes and qualities, so that it can be tested without a renderer.

use moth_sample_protocol::viewer::{Display, FrameKind};

/// Two shapes are the same if their aspect ratios differ by less than this (a natural-log distance,
/// 0.03 is a 3% difference): stretching a frame by that much is not visible.
pub const SAME_SHAPE: f32 = 0.03;
/// A viewer at least this fraction as wide as an output takes the output as it is.
pub const CLOSE_ENOUGH: f32 = 0.85;
/// A viewer at least this fraction as wide as a rendering is served by reducing it.
pub const WORTH_REDUCING: f32 = 0.4;

/// What a viewer asks for in a frame: its size, and the JPEG quality it is compressed at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Want {
	pub size: (u32, u32),
	pub quality: u8,
}

/// What a viewer asks for in a frame of this kind: its canvas times its resolution setting, and for a
/// preview times its preview setting too; and the quality it set for this kind of frame.
pub fn want(display: &Display, kind: FrameKind) -> Want {
	let (scale, quality) = match kind {
		FrameKind::Preview => (display.scale * display.preview, display.preview_quality),
		FrameKind::Final => (display.scale, display.quality),
	};
	let side = |full: u32| ((full as f32 * scale).round() as u32).max(1);
	Want { size: (side(display.width), side(display.height)), quality: quality.clamp(1, 100) as u8 }
}

/// A picture to send: its size (never above the rendering's), its JPEG quality, and the viewers that
/// get it.
#[derive(Debug, Clone, PartialEq)]
pub struct Output<Who> {
	pub size: (u32, u32),
	pub quality: u8,
	pub viewers: Vec<Who>,
}

/// One rendering, and what is made from it. The first output is the rendering itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendering<Who> {
	pub size: (u32, u32),
	pub outputs: Vec<Output<Who>>,
}

fn same_shape(a: (u32, u32), b: (u32, u32)) -> bool {
	((a.0 as f32 / a.1 as f32) / (b.0 as f32 / b.1 as f32)).ln().abs() <= SAME_SHAPE
}

/// Divide the viewers, each with what it asks for, into as few renderings as the rules above allow.
/// Every viewer is in exactly one output.
pub fn plan<Who: Copy>(viewers: &[(Who, Want)]) -> Vec<Rendering<Who>> {
	// biggest first, so that a rendering is always at least as big as everything served from it
	let mut order: Vec<_> = viewers.iter().collect();
	order.sort_by_key(|(_, want)| std::cmp::Reverse(u64::from(want.size.0) * u64::from(want.size.1)));

	let mut renderings: Vec<Rendering<Who>> = Vec::new();
	'viewers: for &(who, Want { size, quality }) in order {
		for rendering in &mut renderings {
			let master = rendering.size;
			if !same_shape(size, master) || (size.0 as f32) < WORTH_REDUCING * master.0 as f32 {
				continue;
			}
			// at most the rendering's size, whatever rounding did to the shape
			let size = (size.0.min(master.0), size.1.min(master.1));
			for output in &mut rendering.outputs {
				if output.quality == quality && size.0 as f32 >= CLOSE_ENOUGH * output.size.0 as f32 {
					output.viewers.push(who);
					continue 'viewers;
				}
			}
			rendering.outputs.push(Output { size, quality, viewers: vec![who] });
			continue 'viewers;
		}
		renderings.push(Rendering { size, outputs: vec![Output { size, quality, viewers: vec![who] }] });
	}
	renderings
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sizes(viewers: &[(u32, (u32, u32))]) -> Vec<Rendering<u32>> {
		plan(&viewers.iter().map(|(who, size)| (*who, Want { size: *size, quality: 90 })).collect::<Vec<_>>())
	}

	fn count_viewers(renderings: &[Rendering<u32>]) -> Vec<u32> {
		let mut all: Vec<u32> = renderings.iter().flat_map(|r| r.outputs.iter().flat_map(|o| o.viewers.iter().copied())).collect();
		all.sort_unstable();
		all
	}

	#[test]
	fn nobody_needs_nothing() {
		assert!(sizes(&[]).is_empty());
	}

	#[test]
	fn one_viewer_gets_a_rendering_of_exactly_its_size() {
		assert_eq!(sizes(&[(1, (1030, 686))]), vec![Rendering { size: (1030, 686), outputs: vec![Output { quality: 90, size: (1030, 686), viewers: vec![1] }] }]);
	}

	#[test]
	fn viewers_of_the_same_size_share_the_rendering_and_the_picture() {
		let plan = sizes(&[(1, (1030, 686)), (2, (1030, 686)), (3, (1030, 686))]);
		assert_eq!(plan.len(), 1);
		assert_eq!(plan[0].outputs.len(), 1);
		assert_eq!(plan[0].outputs[0].viewers.len(), 3);
	}

	#[test]
	fn a_close_size_is_sent_the_same_picture_to_scale_itself() {
		// 900 is 87% of 1030: close enough, no reduction, no second compression
		let plan = sizes(&[(1, (1030, 686)), (2, (900, 600))]);
		assert_eq!(plan.len(), 1);
		assert_eq!(plan[0].outputs, vec![Output { quality: 90, size: (1030, 686), viewers: vec![1, 2] }]);
	}

	#[test]
	fn a_smaller_one_of_the_same_shape_is_served_by_reducing_the_rendering() {
		let plan = sizes(&[(1, (1000, 667)), (2, (600, 400))]);
		assert_eq!(plan.len(), 1, "one rendering");
		assert_eq!(plan[0].size, (1000, 667));
		assert_eq!(plan[0].outputs.len(), 2, "and a reduced copy");
		assert_eq!(plan[0].outputs[1], Output { quality: 90, size: (600, 400), viewers: vec![2] });
	}

	#[test]
	fn reduced_copies_are_shared_by_viewers_near_each_other() {
		let plan = sizes(&[(1, (1000, 667)), (2, (600, 400)), (3, (560, 373)), (4, (510, 340)), (5, (420, 280))]);
		assert_eq!(plan.len(), 1);
		let outputs: Vec<_> = plan[0].outputs.iter().map(|o| (o.size, o.viewers.clone())).collect();
		// 560 and 510 are within 85% of 600; 420 is not, so it is a copy of its own
		assert_eq!(outputs, vec![((1000, 667), vec![1]), ((600, 400), vec![2, 3, 4]), ((420, 280), vec![5])]);
	}

	#[test]
	fn one_too_small_to_be_worth_reducing_gets_a_rendering_of_its_own() {
		let plan = sizes(&[(1, (1000, 667)), (2, (300, 200))]);
		assert_eq!(plan.len(), 2);
		assert_eq!(plan.iter().map(|r| r.size).collect::<Vec<_>>(), vec![(1000, 667), (300, 200)]);
	}

	#[test]
	fn the_edge_of_what_is_worth_reducing_is_forty_percent_of_the_width() {
		assert_eq!(sizes(&[(1, (1000, 667)), (2, (400, 267))]).len(), 1, "40% is reduced");
		assert_eq!(sizes(&[(1, (1000, 667)), (2, (399, 266))]).len(), 2, "just below is not");
	}

	#[test]
	fn a_different_shape_gets_a_rendering_of_its_own() {
		let plan = sizes(&[(1, (1000, 667)), (2, (1000, 400)), (3, (600, 600))]);
		assert_eq!(plan.len(), 3, "the shape of a picture cannot be changed without stretching it");
	}

	#[test]
	fn nearly_the_same_shape_is_the_same_shape() {
		// 1.5015 against 1.5: a 0.1% difference; 1.5 against 1.6: a 6% one
		assert_eq!(sizes(&[(1, (1030, 686)), (2, (600, 400))]).len(), 1);
		assert_eq!(sizes(&[(1, (1000, 667)), (2, (800, 500))]).len(), 2);
	}

	#[test]
	fn the_biggest_viewer_decides_the_rendering_whatever_the_order() {
		let forward = sizes(&[(1, (500, 333)), (2, (1000, 667)), (3, (700, 467))]);
		let backward = sizes(&[(3, (700, 467)), (2, (1000, 667)), (1, (500, 333))]);
		for plan in [forward, backward] {
			assert_eq!(plan.len(), 1);
			assert_eq!(plan[0].size, (1000, 667));
			assert_eq!(plan[0].outputs[0].viewers, vec![2]);
			assert_eq!(count_viewers(&plan), vec![1, 2, 3]);
		}
	}

	#[test]
	fn however_the_sizes_come_every_viewer_is_served_once_by_something_no_bigger_than_its_rendering() {
		// a small deterministic generator, no dependency needed
		let mut state = 0x9E3779B97F4A7C15_u64;
		let mut next = |bound: u32| {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			(state % u64::from(bound)) as u32
		};
		for round in 0..300 {
			let count = 1 + next(12);
			let viewers: Vec<(u32, Want)> = (0..count)
				.map(|id| {
					let shape = [(3, 2), (16, 9), (1, 1), (4, 3), (3, 2)][next(5) as usize];
					let width = 32 + next(1250);
					let quality = [90, 90, 60, 20][next(4) as usize];
					(id, Want { size: (width, (width * shape.1 / shape.0).max(32)), quality })
				})
				.collect();
			let plan = plan(&viewers);
			assert_eq!(count_viewers(&plan), (0..count).collect::<Vec<_>>(), "round {round}: {viewers:?}");
			for rendering in &plan {
				assert_eq!(rendering.outputs[0].size, rendering.size, "the first output is the rendering itself");
				for output in &rendering.outputs {
					assert!(output.size.0 <= rendering.size.0 && output.size.1 <= rendering.size.1, "round {round}");
					assert!(!output.viewers.is_empty());
					for who in &output.viewers {
						let asked = viewers.iter().find(|(id, _)| id == who).unwrap().1;
						assert_eq!(asked.quality, output.quality, "round {round}: a viewer gets the quality it asked for");
					}
				}
			}
			assert!(plan.len() <= viewers.len());
		}
	}

	#[test]
	fn the_size_asked_for_follows_the_canvas_the_resolution_and_the_kind() {
		let size = |display: &Display, kind| want(display, kind).size;
		let display = Display { width: 1000, height: 600, scale: 0.5, ..Display::default() };
		assert_eq!(size(&display, FrameKind::Final), (500, 300));
		assert_eq!(size(&display, FrameKind::Preview), (250, 150), "half of the full frame by default");
		assert_eq!(size(&Display { preview: 0.2, ..display }, FrameKind::Preview), (100, 60), "or what the viewer chose");
		assert_eq!(size(&Display { preview: 0.2, ..display }, FrameKind::Final), (500, 300), "which only the previews follow");
		assert_eq!(size(&Display { width: 64, height: 48, scale: 0.25, ..Display::default() }, FrameKind::Preview), (8, 6), "the shape is kept even when tiny");
		assert_eq!(size(&Display { width: 1, height: 1, scale: 0.25, ..Display::default() }, FrameKind::Preview), (1, 1), "but never nothing");
	}

	#[test]
	fn the_quality_asked_for_is_the_one_set_for_the_kind_of_frame() {
		let display = Display { quality: 95, preview_quality: 30, ..Display::default() };
		assert_eq!(want(&display, FrameKind::Final).quality, 95);
		assert_eq!(want(&display, FrameKind::Preview).quality, 30);
		assert_eq!(want(&Display::default(), FrameKind::Final).quality, 90);
		assert_eq!(want(&Display::default(), FrameKind::Preview).quality, 60);
	}

	fn with_quality(who: u32, size: (u32, u32), quality: u8) -> (u32, Want) {
		(who, Want { size, quality })
	}

	#[test]
	fn viewers_of_the_same_screen_but_another_quality_share_the_rendering_and_not_the_compression() {
		let plan = plan(&[with_quality(1, (640, 400), 90), with_quality(2, (640, 400), 90), with_quality(3, (640, 400), 30)]);
		assert_eq!(plan.len(), 1, "one rendering");
		assert_eq!(plan[0].outputs.len(), 2, "two compressions");
		assert_eq!((plan[0].outputs[0].quality, plan[0].outputs[0].viewers.clone()), (90, vec![1, 2]));
		assert_eq!((plan[0].outputs[1].quality, plan[0].outputs[1].viewers.clone(), plan[0].outputs[1].size), (30, vec![3], (640, 400)));
	}

	#[test]
	fn a_close_size_is_only_sent_the_picture_if_it_wants_the_same_quality() {
		let same = plan(&[with_quality(1, (1000, 667), 90), with_quality(2, (900, 600), 90)]);
		assert_eq!(same[0].outputs.len(), 1, "close and the same quality: the same bytes");
		let other = plan(&[with_quality(1, (1000, 667), 90), with_quality(2, (900, 600), 40)]);
		assert_eq!((other.len(), other[0].outputs.len()), (1, 2), "close but another quality: its own compression, from the same rendering");
		assert_eq!(other[0].outputs[1].size, (900, 600), "at its own size");
	}

	#[test]
	fn reduced_copies_are_shared_only_by_viewers_of_the_same_quality() {
		let plan = plan(&[with_quality(1, (1000, 667), 90), with_quality(2, (600, 400), 50), with_quality(3, (580, 387), 50), with_quality(4, (570, 380), 70)]);
		assert_eq!(plan.len(), 1);
		let outputs: Vec<_> = plan[0].outputs.iter().map(|o| (o.size, o.quality, o.viewers.clone())).collect();
		assert_eq!(outputs, vec![((1000, 667), 90, vec![1]), ((600, 400), 50, vec![2, 3]), ((570, 380), 70, vec![4])]);
	}
}
