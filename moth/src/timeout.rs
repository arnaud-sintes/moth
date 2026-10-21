use wasm_bindgen::{JsCast, closure::Closure};

use crate::Error;

/// RAII `setTimeout`: the callback runs once, after the delay, unless the `Timeout` is dropped
/// first, in which case it never runs. Dropping it after it ran is harmless, and it may be
/// dropped from inside its own callback.
pub struct Timeout {
	id: i32,
	_callback: Closure<dyn FnMut()>,
}

impl Timeout {
	/// Run `callback` once, after `milliseconds` (capped at `i32::MAX`). Fails if there is no
	/// window or the browser refuses the timer.
	pub fn after(milliseconds: u32, callback: impl FnOnce() + 'static) -> Result<Self, Error> {
		let window = web_sys::window().ok_or_else(|| Error::new("no window"))?;
		let callback = Closure::once(callback);
		let id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
			callback.as_ref().unchecked_ref(),
			i32::try_from(milliseconds).unwrap_or(i32::MAX),
		)?;
		Ok(Self { id, _callback: callback })
	}
}

impl Drop for Timeout {
	fn drop(&mut self) {
		if let Some(window) = web_sys::window() {
			window.clear_timeout_with_handle(self.id);
		}
	}
}
