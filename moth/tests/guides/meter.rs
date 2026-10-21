use moth::{Body, Component, Definition, Error, Host, Parent, Prop};
use web_sys::HtmlElement;

/// A bar that is filled to a fraction. It has its own element, with a template and a stylesheet of
/// its own (so its inside is styled by `--meter-*` variables and `::part()`, not by page CSS).
pub struct Meter {
	host: Host,
	level: Prop<f64>,
}

impl Meter {
	const DEFINITION: Definition = Definition {
		tag: "guide-meter",
		template: r#"<div id="track" part="track"><div id="fill" part="fill"></div></div>"#,
		css: "
			:host { display: block; }
			#track { height: 1em; background: var(--meter-track, #ddd); }
			#fill { height: 100%; width: 0; background: var(--meter-fill, steelblue); }
		",
		attributes: &[],
		form_associated: false,
	};

	pub fn new(parent: &impl Parent, level: f64) -> Result<Self, Error> {
		// the element, registered on first use and attached under the parent; the returned host
		// removes it again when this component is dropped
		let host = Self::DEFINITION.create(Some(parent.parent_element()))?;
		let fill: HtmlElement = host.shadow("#fill")?;

		// the bar follows the level
		let level = Prop::new(level.clamp(0.0, 1.0));
		level.bind(move |level| {
			let _ = fill.set_attribute("style", &format!("width: {}%", level * 100.0));
		});
		Ok(Self { host, level })
	}

	pub fn level(&self) -> f64 {
		self.level.get()
	}

	pub fn set_level(&self, level: f64) {
		self.level.set(level.clamp(0.0, 1.0));
	}
}

impl Component for Meter {
	fn host(&self) -> &Host {
		&self.host
	}
}

fn main() {
	let body = Body::new().unwrap();
	let meter = Meter::new(&body, 0.25).unwrap();
	meter.set_level(0.5);
	moth::mount((body, meter)).unwrap();
}
