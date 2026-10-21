use std::cell::RefCell;

use wasm_bindgen::JsValue;
use web_sys::HtmlSelectElement;

use crate::{
	Component, Definition, Error, Host, Listener, Parent, Prop,
	component::{effectively_enabled_now, emit_from},
	elements::bind_form_value,
};

#[derive(Clone, PartialEq)]
struct State {
	options: Vec<String>,
	selected: Option<usize>,
}

/// Drop-down list over a list of text options, form-associated: the selected option's text is
/// submitted under the host's `name` (nothing while it or an ancestor is disabled). `on_changed`
/// is called with the new selection when the user picks an option, not when it is set from code.
///
/// Not available in markup: its options come from Rust.
pub struct Dropdown {
	_change: Listener,
	state: Prop<State>,
	host: Host,
}

impl Dropdown {
	const DEF: Definition = Definition {
		tag: "moth-dropdown",
		template: include_str!("dropdown.html"),
		css: include_str!("dropdown.css"),
		attributes: &[],
		form_associated: true,
	};

	/// Create a dropdown under `parent` showing `options`, with nothing selected. `on_changed`
	/// receives the index of the option the user picked. A user pick also raises the bubbling
	/// `events::SELECT` event on the host, its detail being that index (`null` for none).
	pub fn new(
		parent: &impl Parent,
		options: Vec<String>,
		mut on_changed: impl FnMut(Option<usize>) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		let select: HtmlSelectElement = host.shadow("select")?;
		let control = select.clone();
		host.effective.bind(move |effective| control.set_disabled(!effective));

		let state = Prop::new(State { options, selected: None });
		let control = select.clone();
		let rendered = RefCell::new(None::<Vec<String>>);
		state.bind(move |state| {
			// rebuild the <option>s only when the list itself changed
			if rendered.borrow().as_ref() != Some(&state.options) {
				control.set_text_content(None);
				if let Some(document) = control.owner_document() {
					for text in &state.options {
						if let Ok(option) = document.create_element("option") {
							option.set_text_content(Some(text));
							let _ = control.append_child(&option);
						}
					}
				}
				*rendered.borrow_mut() = Some(state.options.clone());
			}
			control.set_selected_index(state.selected.map_or(-1, |index| index as i32));
		});
		bind_form_value(&host, &state, |state| {
			state.selected.and_then(|index| state.options.get(index).cloned())
		});

		let (weak, control, element) = (state.downgrade(), select.clone(), host.element().clone());
		let change = Listener::new(&select, "change", move |_| {
			// a script can dispatch `change` on a disabled control; a user cannot
			if !effectively_enabled_now(&element) {
				return;
			}
			let selected = usize::try_from(control.selected_index()).ok();
			if let Some(mut current) = weak.get() {
				current.selected = selected;
				weak.set(current);
			}
			let detail = selected.map_or(JsValue::NULL, |index| JsValue::from_f64(index as f64));
			let _ = emit_from(&element, crate::events::SELECT, &detail);
			on_changed(selected);
		})?;
		Ok(Self { _change: change, state, host })
	}

	/// The current options.
	pub fn options(&self) -> Vec<String> {
		self.state.get().options
	}

	/// Replace the options; the selection is kept when it still exists, cleared otherwise.
	pub fn set_options(&self, options: Vec<String>) {
		let mut state = self.state.get();
		state.selected = state.selected.filter(|index| *index < options.len());
		state.options = options;
		self.state.set(state);
	}

	/// The index of the selected option, if any.
	pub fn selected(&self) -> Option<usize> {
		self.state.get().selected
	}

	/// Select by index; an index with no option clears the selection.
	pub fn set_selected(&self, selected: Option<usize>) {
		let mut state = self.state.get();
		state.selected = selected.filter(|index| *index < state.options.len());
		self.state.set(state);
	}

	/// The text of the selected option, if any.
	pub fn selected_text(&self) -> Option<String> {
		let state = self.state.get();
		state.selected.and_then(|index| state.options.get(index).cloned())
	}

	/// The name the value is submitted under (the `name` attribute of the host). Fails if the
	/// browser refuses the attribute.
	pub fn set_name(&self, name: &str) -> Result<(), Error> {
		Ok(self.host.element().set_attribute("name", name)?)
	}
}

impl Component for Dropdown {
	fn host(&self) -> &Host {
		&self.host
	}
}
