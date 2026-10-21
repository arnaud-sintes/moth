use web_sys::HtmlInputElement;

use crate::{
	Component, Declarative, Definition, Error, Host, Listener, Parent, Prop, WeakProp,
	component::effectively_enabled_now, elements::bind_form_value,
};

/// Single-line text input, form-associated: its text is submitted under its `name` (nothing while
/// it or an ancestor is disabled).
///
/// `on_changed` is called with the new text when the user edits it; setting the text from code
/// does not call it. In markup: `<moth-edit name="user" value="..." placeholder="...">`, with
/// the host's ordinary (composed) `input` event for changes.
pub struct Edit {
	_input: Listener,
	text: Prop<String>,
	placeholder: Prop<String>,
	host: Host,
}

impl Edit {
	const DEF: Definition = Definition {
		tag: "moth-edit",
		template: include_str!("edit.html"),
		css: include_str!("edit.css"),
		attributes: &["value", "placeholder"],
		form_associated: true,
	};

	/// Create an edit under `parent` holding `text`. `on_changed` receives the new text each time
	/// the user edits it, not when it is set from code.
	pub fn new(
		parent: &impl Parent,
		text: &str,
		on_changed: impl FnMut(String) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Self::build(host, text.to_owned(), Some(Box::new(on_changed)))
	}

	fn build(
		host: Host,
		initial: String,
		mut on_changed: Option<Box<dyn FnMut(String)>>,
	) -> Result<Self, Error> {
		let input: HtmlInputElement = host.shadow("input")?;
		let control = input.clone();
		host.effective.bind(move |effective| control.set_disabled(!effective));

		let text = Prop::new(initial);
		let control = input.clone();
		text.bind(move |text| control.set_value(text));
		bind_form_value(&host, &text, |text| Some(text.clone()));

		let placeholder = Prop::new(String::new());
		let control = input.clone();
		placeholder.bind(move |placeholder| control.set_placeholder(placeholder));

		let (state, control, element) = (text.downgrade(), input.clone(), host.element().clone());
		let listener = Listener::new(&input, "input", move |_| {
			// a script can dispatch `input` on a disabled control; a user cannot
			if !effectively_enabled_now(&element) {
				return;
			}
			let value = control.value();
			state.set(value.clone());
			if let Some(on_changed) = on_changed.as_mut() {
				on_changed(value);
			}
		})?;
		Ok(Self { _input: listener, text, placeholder, host })
	}

	/// The current text.
	pub fn text(&self) -> String {
		self.text.get()
	}

	/// Replace the text. Never calls `on_changed`.
	pub fn set_text(&self, text: &str) {
		self.text.set(text.to_owned());
	}

	/// Non-owning access to the text, for callbacks: it reports that the edit is gone instead of
	/// touching it.
	pub fn text_handle(&self) -> WeakProp<String> {
		self.text.downgrade()
	}

	/// The current placeholder, empty by default.
	pub fn placeholder(&self) -> String {
		self.placeholder.get()
	}

	/// Set the text shown while the edit is empty.
	pub fn set_placeholder(&self, placeholder: &str) {
		self.placeholder.set(placeholder.to_owned());
	}

	/// The name the value is submitted under (the `name` attribute of the host).
	pub fn set_name(&self, name: &str) -> Result<(), Error> {
		Ok(self.host.element().set_attribute("name", name)?)
	}
}

impl Component for Edit {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Edit {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		Self::build(host, String::new(), None)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		match name {
			"value" => self.set_text(value.unwrap_or_default()),
			"placeholder" => self.set_placeholder(value.unwrap_or_default()),
			_ => {}
		}
	}
}
