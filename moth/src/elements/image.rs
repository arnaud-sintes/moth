use web_sys::HtmlImageElement;

use crate::{Component, Parent, Declarative, Definition, Error, Host, Prop, WeakProp};

/// Image. An empty source clears the `src` attribute instead of requesting the page itself.
/// In markup: `<moth-image src="..." alt="...">`.
pub struct Image {
	source: Prop<String>,
	alt: Prop<String>,
	host: Host,
}

impl Image {
	const DEF: Definition = Definition {
		tag: "moth-image",
		template: include_str!("image.html"),
		css: include_str!("image.css"),
		attributes: &["src", "alt"],
		form_associated: false,
	};

	/// Create an image under `parent` with an empty source and an empty alternative text.
	pub fn new(parent: &impl Parent) -> Result<Self, Error> {
		Self::build(Self::DEF.create(Some(parent.parent_element()))?)
	}

	fn build(host: Host) -> Result<Self, Error> {
		let image: HtmlImageElement = host.shadow("img")?;

		let source = Prop::new(String::new());
		let target = image.clone();
		source.bind(move |source| {
			// attribute name is a constant: a failure is not actionable
			let _ = if source.is_empty() {
				target.remove_attribute("src")
			} else {
				target.set_attribute("src", source)
			};
		});

		let alt = Prop::new(String::new());
		alt.bind(move |alt| image.set_alt(alt));
		Ok(Self { source, alt, host })
	}

	/// The current source (the `src` of the inner image); empty when none is set.
	pub fn source(&self) -> String {
		self.source.get()
	}

	/// Set the source, a URL for the browser to load. An empty string clears the `src` attribute.
	pub fn set_source(&self, source: &str) {
		self.source.set(source.to_owned());
	}

	/// Non-owning access to the source, safe to move into callbacks (e.g. a streaming socket).
	pub fn source_handle(&self) -> WeakProp<String> {
		self.source.downgrade()
	}

	/// The current alternative text.
	pub fn alt(&self) -> String {
		self.alt.get()
	}

	/// Set the alternative text of the inner image.
	pub fn set_alt(&self, alt: &str) {
		self.alt.set(alt.to_owned());
	}
}

impl Component for Image {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Image {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		Self::build(host)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		match name {
			"src" => self.set_source(value.unwrap_or_default()),
			"alt" => self.set_alt(value.unwrap_or_default()),
			_ => {}
		}
	}
}
