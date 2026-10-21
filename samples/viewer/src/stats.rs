//! What the HUD shows, computed from the frames that arrived: pure arithmetic, no browser.

use std::collections::VecDeque;

use moth_sample_protocol::viewer::{FrameHeader, FrameKind};

/// The window over which the rates are measured.
const WINDOW_MS: f64 = 1000.0;
/// Latency samples kept for the sparkline.
const HISTORY: usize = 90;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastFrame {
	pub kind: FrameKind,
	pub width: u16,
	pub height: u16,
	pub bytes: usize,
	pub render_ms: f64,
	pub encode_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
	/// Frames received in the last second.
	pub fps: f64,
	/// Megabits per second received over the last second.
	pub megabits: f64,
	/// A smoothed round trip, from the camera moving to its frame arriving; `None` before the first.
	pub latency_ms: Option<f64>,
	pub last: Option<LastFrame>,
}

#[derive(Debug, Default)]
pub struct Stats {
	/// (arrival time in ms, bytes) of the frames inside the window.
	arrivals: VecDeque<(f64, usize)>,
	latency: Option<f64>,
	history: VecDeque<f64>,
	last: Option<LastFrame>,
}

impl Stats {
	pub fn frame(&mut self, now_ms: f64, bytes: usize, header: &FrameHeader) {
		self.arrivals.push_back((now_ms, bytes));
		self.expire(now_ms);
		self.last = Some(LastFrame {
			kind: header.kind,
			width: header.width,
			height: header.height,
			bytes,
			render_ms: f64::from(header.render_us) / 1000.0,
			encode_ms: f64::from(header.encode_us) / 1000.0,
		});
	}

	/// A measured round trip. Smoothed for the number, kept raw for the sparkline.
	pub fn latency(&mut self, ms: f64) {
		if !ms.is_finite() || ms < 0.0 {
			return;
		}
		self.latency = Some(self.latency.map_or(ms, |previous| previous * 0.8 + ms * 0.2));
		if self.history.len() == HISTORY {
			self.history.pop_front();
		}
		self.history.push_back(ms);
	}

	pub fn snapshot(&mut self, now_ms: f64) -> Snapshot {
		self.expire(now_ms);
		let bytes: usize = self.arrivals.iter().map(|(_, bytes)| bytes).sum();
		Snapshot {
			fps: self.arrivals.len() as f64 * 1000.0 / WINDOW_MS,
			megabits: bytes as f64 * 8.0 / 1e6 * 1000.0 / WINDOW_MS,
			latency_ms: self.latency,
			last: self.last,
		}
	}

	pub fn history(&self) -> impl Iterator<Item = f64> + '_ {
		self.history.iter().copied()
	}

	fn expire(&mut self, now_ms: f64) {
		while self.arrivals.front().is_some_and(|(at, _)| now_ms - at > WINDOW_MS) {
			self.arrivals.pop_front();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn header(kind: FrameKind) -> FrameHeader {
		FrameHeader { seq: 0, view_id: 0, kind, width: 640, height: 480, render_us: 2500, encode_us: 4100 }
	}

	#[test]
	fn nothing_has_arrived_yet() {
		let snapshot = Stats::default().snapshot(0.0);
		assert_eq!((snapshot.fps, snapshot.megabits), (0.0, 0.0));
		assert!(snapshot.latency_ms.is_none() && snapshot.last.is_none());
	}

	#[test]
	fn the_rates_cover_the_last_second_only() {
		let mut stats = Stats::default();
		for step in 0..30 {
			stats.frame(f64::from(step) * 10.0, 12_500, &header(FrameKind::Preview)); // 30 frames in 300 ms
		}
		let busy = stats.snapshot(300.0);
		assert_eq!(busy.fps, 30.0);
		assert!((busy.megabits - 3.0).abs() < 1e-9, "30 x 12.5 kB = 3 Mbit in the window: {}", busy.megabits);

		let later = stats.snapshot(900.0); // still inside the second
		assert_eq!(later.fps, 30.0);
		let idle = stats.snapshot(1400.0); // all but the last few have expired
		assert!(idle.fps < 30.0);
		let silent = stats.snapshot(5000.0);
		assert_eq!((silent.fps, silent.megabits), (0.0, 0.0), "a stream that stopped reads zero, not its last rate");
		assert!(silent.last.is_some(), "but the last frame is still remembered");
	}

	#[test]
	fn the_last_frame_is_described_in_milliseconds_and_bytes() {
		let mut stats = Stats::default();
		stats.frame(0.0, 5280, &header(FrameKind::Final));
		let last = stats.snapshot(0.0).last.unwrap();
		assert_eq!(last.kind, FrameKind::Final);
		assert_eq!((last.width, last.height, last.bytes), (640, 480, 5280));
		assert!((last.render_ms - 2.5).abs() < 1e-9 && (last.encode_ms - 4.1).abs() < 1e-9);
	}

	#[test]
	fn latency_is_smoothed_for_the_number_and_raw_for_the_history() {
		let mut stats = Stats::default();
		stats.latency(40.0);
		assert_eq!(stats.snapshot(0.0).latency_ms, Some(40.0), "the first sample is taken as it is");
		stats.latency(100.0);
		let smoothed = stats.snapshot(0.0).latency_ms.unwrap();
		assert!((smoothed - 52.0).abs() < 1e-9, "{smoothed}");
		assert_eq!(stats.history().collect::<Vec<_>>(), [40.0, 100.0]);
	}

	#[test]
	fn the_history_is_bounded_and_drops_the_oldest() {
		let mut stats = Stats::default();
		for sample in 0..(HISTORY + 25) {
			stats.latency(sample as f64);
		}
		let history: Vec<f64> = stats.history().collect();
		assert_eq!(history.len(), HISTORY);
		assert_eq!(history[0], 25.0);
		assert_eq!(*history.last().unwrap(), (HISTORY + 24) as f64);
	}

	#[test]
	fn a_nonsensical_latency_is_ignored() {
		let mut stats = Stats::default();
		stats.latency(f64::NAN);
		stats.latency(-5.0);
		stats.latency(f64::INFINITY);
		assert!(stats.snapshot(0.0).latency_ms.is_none());
		assert_eq!(stats.history().count(), 0);
	}
}
