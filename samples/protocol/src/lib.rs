//! JSON messages exchanged by the sample front-end and back-end, shared so neither side can drift.
//! Every field is optional and omitted when empty, like the original moth sample.

use serde::{Deserialize, Serialize};

pub mod todo;
pub mod viewer;

/// Front-end to back-end.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontToBack {
	/// An informative message to log.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
	/// Start (`true`) or stop (`false`) the image stream.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub stream: Option<bool>,
}

/// Back-end to front-end.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackToFront {
	/// An informative message to log.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
	/// An image as a `data:` URL.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub image: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_fields_are_omitted_like_the_original_wire_format() {
		let json = serde_json::to_string(&FrontToBack { stream: Some(true), ..Default::default() })
			.unwrap();
		assert_eq!(json, r#"{"stream":true}"#);
	}

	#[test]
	fn missing_fields_deserialize_to_none() {
		let data: BackToFront = serde_json::from_str(r#"{"message":"hi"}"#).unwrap();
		assert_eq!(data, BackToFront { message: Some("hi".into()), image: None });
	}
}
