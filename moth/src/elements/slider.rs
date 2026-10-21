use web_sys::HtmlInputElement;

use crate::{
	Component, Declarative, Definition, Error, Host, Listener, Parent, Prop, WeakProp,
	component::{effectively_enabled_now, parse_html_number},
	elements::bind_form_value,
};

/// Range slider, form-associated (its value is submitted under its `name`, nothing while it or an
/// ancestor is disabled).
///
/// The value stays within `min..=max` and on the browser's step grid: setting any of the four
/// re-clamps and re-snaps it, so the property is always what the input displays and what the form
/// submits. `on_changed` is called when the user moves the slider, not when the value is set from
/// code. In markup: `<moth-slider value min max step name>`.
pub struct Slider {
	_input: Listener,
	input: HtmlInputElement,
	value: Prop<f64>,
	min: Prop<f64>,
	max: Prop<f64>,
	step: Prop<f64>,
	host: Host,
}

/// Not `f64::clamp`, which panics when min > max. Like an HTML range input, an inverted range
/// counts as max = min; a NaN lands on `min`.
fn clamp_range(value: f64, min: f64, max: f64) -> f64 {
	value.max(min).min(max.max(min))
}

impl Slider {
	const DEF: Definition = Definition {
		tag: "moth-slider",
		template: include_str!("slider.html"),
		css: include_str!("slider.css"),
		// order matters: initial attributes are applied in this order, and the range must be set
		// before the value is clamped against it
		attributes: &["min", "max", "step", "value"],
		form_associated: true,
	};

	/// A slider over 0..=100 by steps of 1, like a bare `<input type=range>`, at `value`.
	pub fn new(
		parent: &impl Parent,
		value: f64,
		on_changed: impl FnMut(f64) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		Self::build(host, value, Some(Box::new(on_changed)))
	}

	fn build(
		host: Host,
		initial: f64,
		mut on_changed: Option<Box<dyn FnMut(f64)>>,
	) -> Result<Self, Error> {
		let input: HtmlInputElement = host.shadow("input")?;
		let control = input.clone();
		host.effective.bind(move |effective| control.set_disabled(!effective));

		// range attributes first: the browser clamps `value` against them
		let (min, max, step) = (Prop::new(0.0), Prop::new(100.0), Prop::new(1.0));
		for (prop, name) in [(&min, "min"), (&max, "max"), (&step, "step")] {
			let control = input.clone();
			prop.bind(move |value| {
				// attribute names are constants: a failure is not actionable
				let _ = control.set_attribute(name, &value.to_string());
			});
		}

		let value = Prop::new(clamp_range(initial, 0.0, 100.0));
		// whatever sets the value (a setter, a handle), the property ends up as the input shows it:
		// the browser snaps to the step grid when the value is written
		let (control, state) = (input.clone(), value.downgrade());
		value.bind(move |value| {
			control.set_value(&value.to_string());
			let shown = control.value_as_number();
			if shown.is_finite() && shown != *value {
				state.set(shown);
			}
		});
		bind_form_value(&host, &value, |value| Some(value.to_string()));

		let (state, control, element) = (value.downgrade(), input.clone(), host.element().clone());
		let listener = Listener::new(&input, "input", move |_| {
			// a script can dispatch `input` on a disabled control; a user cannot
			if !effectively_enabled_now(&element) {
				return;
			}
			let moved = control.value_as_number();
			state.set(moved);
			if let Some(on_changed) = on_changed.as_mut() {
				on_changed(moved);
			}
		})?;
		Ok(Self { _input: listener, input, value, min, max, step, host })
	}

	/// The current value, as the input shows it (within the range and on the step grid).
	pub fn value(&self) -> f64 {
		self.value.get()
	}

	/// Set the value, clamped to `min..=max` and then snapped by the browser to the step grid.
	/// Never calls `on_changed`.
	pub fn set_value(&self, value: f64) {
		self.value.set(self.clamped(value));
	}

	/// Non-owning access for callbacks. It clamps like [`Slider::set_value`].
	pub fn handle(&self) -> SliderHandle {
		SliderHandle { value: self.value.downgrade(), min: self.min.downgrade(), max: self.max.downgrade() }
	}

	/// Set the lower bound; the value is re-clamped and re-snapped if needed. The argument is not
	/// validated.
	pub fn set_min(&self, min: f64) {
		self.min.set(min);
		self.settle();
	}

	/// Set the upper bound; the value is re-clamped and re-snapped if needed. The argument is not
	/// validated, and a bound below `min` counts as `min`.
	pub fn set_max(&self, max: f64) {
		self.max.set(max);
		self.settle();
	}

	/// Set the step of the grid the value snaps to; the value is re-snapped if needed. The argument
	/// is not validated.
	pub fn set_step(&self, step: f64) {
		self.step.set(step);
		self.settle();
	}

	/// The name the value is submitted under (the `name` attribute of the host).
	pub fn set_name(&self, name: &str) -> Result<(), Error> {
		Ok(self.host.element().set_attribute("name", name)?)
	}

	fn clamped(&self, value: f64) -> f64 {
		clamp_range(value, self.min.get(), self.max.get())
	}

	/// The range or the step changed under the value: clamp it, then take what the input shows
	/// after the browser re-snapped it.
	fn settle(&self) {
		self.value.set(self.clamped(self.value.get()));
		let shown = self.input.value_as_number();
		if shown.is_finite() {
			self.value.set(shown);
		}
	}
}

impl Component for Slider {
	fn host(&self) -> &Host {
		&self.host
	}
}

/// Non-owning access to a [`Slider`]'s value, safe to keep in callbacks. Unlike a raw
/// [`WeakProp`] it keeps the value within the slider's range.
#[derive(Clone)]
pub struct SliderHandle {
	value: WeakProp<f64>,
	min: WeakProp<f64>,
	max: WeakProp<f64>,
}

impl SliderHandle {
	/// The slider's value, or `None` if the slider is gone.
	pub fn value(&self) -> Option<f64> {
		self.value.get()
	}

	/// Returns whether the slider was still alive.
	pub fn set_value(&self, value: f64) -> bool {
		match (self.min.get(), self.max.get()) {
			(Some(min), Some(max)) => self.value.set(clamp_range(value, min, max)),
			_ => false,
		}
	}
}

impl Declarative for Slider {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		Self::build(host, 50.0, None)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		let Some(number) = value.and_then(parse_html_number) else { return };
		match name {
			"min" => self.set_min(number),
			"max" => self.set_max(number),
			"step" => self.set_step(number),
			"value" => self.set_value(number),
			_ => {}
		}
	}
}
