use crate::{
	Component, Declarative, Definition, Error, Host, Listener, Parent, Prop, WeakProp,
	component::{bind_text, effectively_enabled_now},
	elements::attach_button,
};

/// Push button; `on_click` lives exactly as long as the button and is not called while the
/// button is disabled (itself or through an ancestor).
///
/// In markup (`<moth-push-button text="...">`) there is no Rust callback: listen to the host's
/// ordinary `click` event.
pub struct PushButton {
	_click: Option<Listener>,
	text: Prop<String>,
	host: Host,
}

impl PushButton {
	const DEF: Definition = Definition {
		tag: "moth-push-button",
		template: include_str!("push_button.html"),
		css: include_str!("push_button.css"),
		attributes: &["text"],
		form_associated: false,
	};

	/// Create a button under `parent` labelled `text`. `on_click` is called for each click while the
	/// button is effectively enabled, and never after the button is dropped.
	pub fn new(
		parent: &impl Parent,
		text: &str,
		on_click: impl FnMut() + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Self::build(host, text.to_owned(), true, Some(Box::new(on_click)))
	}

	fn build(
		host: Host,
		text: String,
		apply_initial: bool,
		on_click: Option<Box<dyn FnMut()>>,
	) -> Result<Self, Error> {
		attach_button(&host)?;
		let text = Prop::new(text);
		bind_text(&text, host.element(), apply_initial);

		// `click` is composed, so it is retargeted to the host when fired from the shadow
		// <button>. Programmatic `host.click()` bypasses the inner button's own disabled state,
		// hence the explicit check, which reads the DOM so that it cannot be stale.
		let click = on_click
			.map(|mut on_click| {
				let element = host.element().clone();
				Listener::new(host.element(), "click", move |_| {
					if effectively_enabled_now(&element) {
						on_click();
					}
				})
			})
			.transpose()?;
		Ok(Self { _click: click, text, host })
	}

	/// The current label text.
	pub fn text(&self) -> String {
		self.text.get()
	}

	/// Replace the label text.
	pub fn set_text(&self, text: &str) {
		self.text.set(text.to_owned());
	}

	/// Non-owning access to the label text, for callbacks: it reports that the button is gone
	/// instead of touching it.
	pub fn text_handle(&self) -> WeakProp<String> {
		self.text.downgrade()
	}
}

impl Component for PushButton {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for PushButton {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		let text = host.element().text_content().unwrap_or_default();
		Self::build(host, text, false, None)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		if let ("text", Some(value)) = (name, value) {
			self.set_text(value);
		}
	}
}
