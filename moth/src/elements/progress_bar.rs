use crate::{
	Component, Declarative, Definition, Error, Host, Parent, Prop, WeakProp,
	component::parse_html_number,
};

/// Progress bar over `0..=max`; no value means indeterminate. In markup:
/// `<moth-progress-bar value="0.4" max="1">`.
pub struct ProgressBar {
	value: Prop<Option<f64>>,
	max: Prop<f64>,
	host: Host,
}

/// Not `f64::clamp`: a NaN would pass through it into the attribute.
fn clamp_progress(value: f64, max: f64) -> f64 {
	value.max(0.0).min(max)
}

impl ProgressBar {
	const DEF: Definition = Definition {
		tag: "moth-progress-bar",
		template: include_str!("progress_bar.html"),
		css: include_str!("progress_bar.css"),
		// `max` before `value`: see Slider
		attributes: &["max", "value"],
		form_associated: false,
	};

	/// Indeterminate, over `0..=1`.
	pub fn new(parent: &impl Parent) -> Result<Self, Error> {
		Self::build(Self::DEF.create(Some(parent.parent_element()))?)
	}

	fn build(host: Host) -> Result<Self, Error> {
		let progress: web_sys::Element = host.shadow("progress")?;

		// `max` first: the browser clamps `value` against it
		let max = Prop::new(1.0);
		let target = progress.clone();
		max.bind(move |max| {
			// attribute name is a constant: a failure is not actionable
			let _ = target.set_attribute("max", &max.to_string());
		});

		let value = Prop::<Option<f64>>::new(None);
		value.bind(move |value| {
			let _ = match value {
				Some(value) => progress.set_attribute("value", &value.to_string()),
				None => progress.remove_attribute("value"),
			};
		});
		Ok(Self { value, max, host })
	}

	/// The current value, or `None` when the bar is indeterminate.
	pub fn value(&self) -> Option<f64> {
		self.value.get()
	}

	/// Set the progress, kept within `0..=max`; `None` makes the bar indeterminate.
	pub fn set_value(&self, value: Option<f64>) {
		self.value.set(value.map(|value| clamp_progress(value, self.max.get())));
	}

	/// Non-owning access for callbacks (e.g. a socket's message handler). It clamps like
	/// [`ProgressBar::set_value`].
	pub fn handle(&self) -> ProgressHandle {
		ProgressHandle { value: self.value.downgrade(), max: self.max.downgrade() }
	}

	/// The upper bound, 1 by default.
	pub fn max(&self) -> f64 {
		self.max.get()
	}

	/// The upper bound; must be finite and greater than zero, otherwise an error is returned and
	/// nothing changes. A current value above the new bound is lowered to it.
	pub fn set_max(&self, max: f64) -> Result<(), Error> {
		if !(max.is_finite() && max > 0.0) {
			return Err(Error::new(format!("progress max must be finite and > 0, got {max}")));
		}
		self.max.set(max);
		self.value.set(self.value.get().map(|value| clamp_progress(value, max)));
		Ok(())
	}
}

impl Component for ProgressBar {
	fn host(&self) -> &Host {
		&self.host
	}
}

/// Non-owning access to a [`ProgressBar`]'s value, safe to keep in callbacks. Unlike a raw
/// [`WeakProp`] it keeps the value within `0..=max`.
#[derive(Clone)]
pub struct ProgressHandle {
	value: WeakProp<Option<f64>>,
	max: WeakProp<f64>,
}

impl ProgressHandle {
	/// The bar's value (`None` inside when it is indeterminate), or `None` if the bar is gone.
	pub fn value(&self) -> Option<Option<f64>> {
		self.value.get()
	}

	/// Returns whether the bar was still alive.
	pub fn set_value(&self, value: Option<f64>) -> bool {
		match self.max.get() {
			Some(max) => self.value.set(value.map(|value| clamp_progress(value, max))),
			None => false,
		}
	}
}

impl Declarative for ProgressBar {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		Self::build(host)
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		let number = value.and_then(parse_html_number);
		match name {
			"max" => {
				if let Some(max) = number {
					// an invalid bound in markup is ignored like any invalid attribute value
					let _ = self.set_max(max);
				}
			}
			"value" => self.set_value(number),
			_ => {}
		}
	}
}
