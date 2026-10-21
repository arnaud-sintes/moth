use moth::{Component, Declarative, Definition, Error, Host};
use web_sys::HtmlElement;

/// A number in a pill, for pages that write `<guide-badge count="3"></guide-badge>`.
pub struct Badge {
	host: Host,
	shown: HtmlElement,
}

impl Badge {
	/// Show a number (what the `count` attribute does too).
	pub fn set_count(&self, count: u32) {
		self.shown.set_text_content(Some(&count.to_string()));
	}
}

impl Component for Badge {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Declarative for Badge {
	const DEFINITION: Definition = Definition {
		tag: "guide-badge",
		template: r#"<span id="shown" part="count">0</span>"#,
		css: ":host { display: inline-block; }",
		// the attributes reported to `attribute_changed`
		attributes: &["count"],
		form_associated: false,
	};

	/// The element is in the page and moth has given it a host: build the component around it.
	fn attach(host: Host) -> Result<Self, Error> {
		let shown = host.shadow("#shown")?;
		Ok(Self { host, shown })
	}

	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		if let ("count", Some(count)) = (name, value.and_then(|value| value.parse().ok())) {
			self.set_count(count);
		}
	}
}

fn main() {
	// make the stock elements and this one usable in markup; those already in the page are upgraded
	moth::define_all().unwrap();
	moth::define::<Badge>().unwrap();
}
