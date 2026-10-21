use crate::{Component, Parent, Declarative, Definition, Error, Host};

/// Generic container: its children are projected through a `<slot>`. Lay it out with CSS, either
/// on the element or through `--moth-panel-display` (default `block`) and `--moth-panel-gap`.
/// In markup: `<moth-panel>`.
pub struct Panel {
	host: Host,
}

impl Panel {
	const DEF: Definition = Definition {
		tag: "moth-panel",
		template: include_str!("panel.html"),
		css: include_str!("panel.css"),
		attributes: &[],
		form_associated: false,
	};

	/// Create an empty panel under `parent`. Dropping it removes its element, and the components
	/// created under it with it.
	pub fn new(parent: &impl Parent) -> Result<Self, Error> {
		Ok(Self { host: Self::DEF.create(Some(parent.parent_element()))? })
	}
}

impl Component for Panel {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Panel {
	const DEFINITION: Definition = Self::DEF;

	fn attach(host: Host) -> Result<Self, Error> {
		Ok(Self { host })
	}
}
