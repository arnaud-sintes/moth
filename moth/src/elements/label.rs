use crate::{
	Component, Parent, Declarative, Definition, Error, Host, Prop, WeakProp, component::bind_text,
};

/// Inline text. In markup: `<moth-label text="..."></moth-label>`, or plain content.
pub struct Label {
	text: Prop<String>,
	host: Host,
}

impl Label {
	const DEF: Definition = Definition {
		tag: "moth-label",
		template: include_str!("label.html"),
		css: include_str!("label.css"),
		attributes: &["text"],
		form_associated: false,
	};

	/// Create a label under `parent` showing `text`. The text lives in a text node of its own, so
	/// children added to the label are not affected by text changes.
	pub fn new(parent: &impl Parent, text: &str) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Ok(Self::build(host, text.to_owned(), true))
	}

	fn build(host: Host, text: String, apply_initial: bool) -> Self {
		let text = Prop::new(text);
		bind_text(&text, host.element(), apply_initial);
		Self { text, host }
	}

	/// The current text.
	pub fn text(&self) -> String {
		self.text.get()
	}

	/// Replace the text shown.
	pub fn set_text(&self, text: &str) {
		self.text.set(text.to_owned());
	}

	/// Non-owning access to the text, safe to move into callbacks.
	pub fn text_handle(&self) -> WeakProp<String> {
		self.text.downgrade()
	}
}

impl Component for Label {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Label {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		// the element's own content is the text, left untouched until a change
		let text = host.element().text_content().unwrap_or_default();
		Ok(Self::build(host, text, false))
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		if let ("text", Some(value)) = (name, value) {
			self.set_text(value);
		}
	}
}
