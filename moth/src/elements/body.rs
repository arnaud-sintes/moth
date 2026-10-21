use crate::{Component, Definition, Error, Host};

/// Root element: a block container attached to the document `<body>`.
pub struct Body {
	host: Host,
}

impl Body {
	const DEF: Definition = Definition {
		tag: "moth-body",
		template: include_str!("body.html"),
		css: include_str!("body.css"),
		attributes: &[],
		form_associated: false,
	};

	/// Create the root element, appended to the document `<body>`. Dropping the `Body` removes it,
	/// and with it everything created under it. Fails if there is no document or `<body>`, or if
	/// `moth-body` is already defined by something that is not moth.
	pub fn new() -> Result<Self, Error> {
		Ok(Self { host: Self::DEF.create(None)? })
	}
}

impl Component for Body {
	fn host(&self) -> &Host {
		&self.host
	}
}
