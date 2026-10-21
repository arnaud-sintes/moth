use std::fmt;

use wasm_bindgen::{JsCast, JsValue};

/// Error type for every fallible moth operation. moth never panics on a DOM failure.
#[derive(Debug, Clone)]
pub struct Error(String);

impl Error {
	/// An error with the given message, which is what [`Display`](std::fmt::Display) prints.
	pub fn new(message: impl Into<String>) -> Self {
		Self(message.into())
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for Error {}

impl From<JsValue> for Error {
	fn from(value: JsValue) -> Self {
		if let Some(error) = value.dyn_ref::<js_sys::Error>() {
			// also covers DOMException (e.g. `SyntaxError: The URL 'x' is invalid`)
			return Self(format!("{}: {}", error.name(), error.message()));
		}
		Self(value.as_string().unwrap_or_else(|| format!("{value:?}")))
	}
}
