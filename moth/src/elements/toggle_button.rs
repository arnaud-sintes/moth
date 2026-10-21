use wasm_bindgen::JsValue;

use crate::{
	Component, Declarative, Definition, Error, Host, Listener, Parent, Prop, WeakProp,
	component::{bind_text, effectively_enabled_now, emit_from, set_flag},
	elements::attach_button,
};

/// Button holding a boolean state, reflected as the `toggled` attribute of the host and as
/// `aria-pressed` on the inner button. Setting the state from code never calls the callback.
///
/// - [`ToggleButton::new`]: a click flips the state at once and calls `on_toggled` with the new
///   value. Simple, but if the state really lives elsewhere (a replicated model) and that owner
///   refuses or never answers, the button keeps the value it flipped to.
/// - [`ToggleButton::controlled`]: a click does not flip anything, it only calls `on_request` with
///   the state it asks for; the state changes when the owner calls [`ToggleButton::set_toggled`].
///   The button can then never disagree with its owner.
///
/// In markup (`<moth-toggle-button text="..." toggled>`) there is no Rust callback: that
/// attribute is the state.
pub struct ToggleButton {
	_click: Listener,
	toggled: Prop<bool>,
	text: Prop<String>,
	host: Host,
}

impl ToggleButton {
	const DEF: Definition = Definition {
		tag: "moth-toggle-button",
		template: include_str!("push_button.html"),
		css: include_str!("toggle_button.css"),
		attributes: &["text", "toggled"],
		form_associated: false,
	};

	/// Create a button under `parent` labelled `text`, not toggled. A click flips the state and
	/// calls `on_toggled` with the new value.
	pub fn new(
		parent: &impl Parent,
		text: &str,
		on_toggled: impl FnMut(bool) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Self::build(host, text.to_owned(), true, false, Some(Box::new(on_toggled)))
	}

	/// Create a button under `parent` labelled `text`, not toggled, whose state its owner controls.
	/// A click leaves the state alone and calls `on_request` with the state it asks for.
	pub fn controlled(
		parent: &impl Parent,
		text: &str,
		on_request: impl FnMut(bool) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Self::build(host, text.to_owned(), true, true, Some(Box::new(on_request)))
	}

	fn build(
		host: Host,
		text: String,
		apply_initial: bool,
		controlled: bool,
		mut callback: Option<Box<dyn FnMut(bool)>>,
	) -> Result<Self, Error> {
		let button = attach_button(&host)?;
		let text = Prop::new(text);
		bind_text(&text, host.element(), apply_initial);

		let toggled = Prop::new(host.element().has_attribute("toggled"));
		let reflected = host.element().clone();
		toggled.bind(move |toggled| {
			set_flag(&reflected, "toggled", *toggled);
			// attribute name and value are constants: a failure is not actionable
			let _ = button.set_attribute("aria-pressed", if *toggled { "true" } else { "false" });
		});

		let (element, state) = (host.element().clone(), toggled.downgrade());
		let click = Listener::new(host.element(), "click", move |_| {
			if !effectively_enabled_now(&element) {
				return;
			}
			let Some(current) = state.get() else { return };
			if !controlled {
				state.set(!current);
			}
			// before the callback, which may drop the button: the ancestors still see it attached
			let _ = emit_from(&element, crate::events::TOGGLE, &JsValue::from_bool(!current));
			if let Some(callback) = callback.as_mut() {
				callback(!current);
			}
		})?;
		Ok(Self { _click: click, toggled, text, host })
	}

	/// The current state.
	pub fn toggled(&self) -> bool {
		self.toggled.get()
	}

	/// Set the state. Never calls the callback.
	pub fn set_toggled(&self, toggled: bool) {
		self.toggled.set(toggled);
	}

	/// Non-owning access to the state, for callbacks: it reports that the button is gone instead
	/// of touching it.
	pub fn toggled_handle(&self) -> WeakProp<bool> {
		self.toggled.downgrade()
	}

	/// The current label text.
	pub fn text(&self) -> String {
		self.text.get()
	}

	/// Replace the label text.
	pub fn set_text(&self, text: &str) {
		self.text.set(text.to_owned());
	}
}

impl Component for ToggleButton {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for ToggleButton {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		let text = host.element().text_content().unwrap_or_default();
		Self::build(host, text, false, false, None)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		match (name, value) {
			("text", Some(value)) => self.set_text(value),
			("toggled", value) => self.set_toggled(value.is_some()),
			_ => {}
		}
	}
}
