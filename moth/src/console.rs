//! Browser console output.

/// Write `message` to the browser console at the `log` level.
pub fn log(message: &str) {
	web_sys::console::log_1(&message.into());
}

/// Write `message` to the browser console at the `warn` level.
pub fn warn(message: &str) {
	web_sys::console::warn_1(&message.into());
}

/// Write `message` to the browser console at the `error` level.
pub fn error(message: &str) {
	web_sys::console::error_1(&message.into());
}
