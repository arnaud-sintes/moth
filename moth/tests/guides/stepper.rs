use moth::{
	Body, Component, ElementRef, Error, Host, Label, Panel, Parent, Prop, PushButton, event_detail,
};
use wasm_bindgen::JsValue;

/// Raised (bubbling and composed) with the new value as its detail when the user changes a stepper.
const CHANGED: &str = "stepper-change";

/// A number with a button on each side. It is a panel as far as the rest of the page can tell: a
/// parent for other components, with an `enabled` state, and removed from the page when dropped.
pub struct Stepper {
	panel: Panel,
	value: Prop<i32>,
	_less: PushButton,
	_label: Label,
	_more: PushButton,
}

impl Stepper {
	pub fn new(parent: &impl Parent, start: i32) -> Result<Self, Error> {
		let panel = Panel::new(parent)?;
		panel.element().set_class_name("stepper");
		let value = Prop::new(start);

		// what a button does: change the value and tell whoever is listening above
		let step = |by: i32| {
			let (value, here) = (value.downgrade(), ElementRef::new(panel.element()));
			move || {
				if let Some(now) = value.get() {
					value.set(now + by);
					let _ = here.emit(CHANGED, &JsValue::from(now + by));
				}
			}
		};
		let less = PushButton::new(&panel, "-", step(-1))?;
		let label = Label::new(&panel, &start.to_string())?;
		let more = PushButton::new(&panel, "+", step(1))?;

		// the number on screen follows the value
		let text = label.text_handle();
		value.bind(move |value| {
			text.set(value.to_string());
		});
		Ok(Self { panel, value, _less: less, _label: label, _more: more })
	}

	pub fn value(&self) -> i32 {
		self.value.get()
	}

	/// Set the value from code (this does not raise the event: that is for what the user does).
	pub fn set_value(&self, value: i32) {
		self.value.set(value);
	}
}

impl Component for Stepper {
	fn host(&self) -> &Host {
		self.panel.host()
	}
}

fn main() {
	let body = Body::new().unwrap();
	let status = Label::new(&body, "quantity: 1").unwrap();
	let stepper = Stepper::new(&body, 1).unwrap();

	// the stepper knows nothing of the label: it raises an event, and whoever contains it may listen
	let text = status.text_handle();
	let listener = body
		.on(CHANGED, move |event| {
			let value = event_detail(&event).and_then(|detail| detail.as_f64()).unwrap_or(0.0);
			text.set(format!("quantity: {value}"));
		})
		.unwrap();

	moth::mount((body, status, stepper, listener)).unwrap();
}
