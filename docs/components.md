# Writing your own components

There are three ways to make a component of your own, from the simplest up. Pick the first that does what you need:

1. **Compose** stock components into a new one (a row, a card, a stepper). Plain Rust, nothing to register.
2. **A new element**: your own custom element with its own template and stylesheet, created from Rust.
3. **An element for markup**: the same, for pages that write `<my-badge count="3">` in HTML.

All three follow the same rules, listed at the end.

## What a component is

A component is a Rust value that owns a **host**: the DOM element that represents it. It implements one trait, `Component`, whose only required method returns the host:

```rust
pub trait Component {
    fn host(&self) -> &Host;
    // ... and provided methods: element(), enabled(), set_enabled(), emit(), on() ...
}
```

Everything else follows from that. Because every `Component` is a `Parent`, other components can be created under it (`Label::new(&my_component, "...")`). Because the host removes its element when it is dropped, **dropping a component removes its element, and with it everything that was created under it**; you write no teardown. And every component has the `enabled` property: disabling one disables the whole subtree under it.

## 1. Compose

A `Stepper` is a number with a button on each side. It is a panel as far as the rest of the page can tell, so it implements `Component` by handing out the panel's host:

```rust
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
```

What to notice:

- **The struct owns its parts.** `_less`, `_label`, `_more` are never read; they are there so that the buttons live as long as the stepper and go with it. Fields drop in the order they are declared, and the order does not matter: removing an element that is already gone is harmless.
- **Callbacks hold handles, not components.** `step` captures `value.downgrade()` (a `WeakProp`) and an `ElementRef` to the panel's element: neither keeps anything alive, and neither can dangle.
- **State is a `Prop`, and the screen follows it.** The label is bound to `value` with `bind`, so changing the value from anywhere (a button, `set_value`) updates the number.
- **A component tells its surroundings with an event, not by calling them.** `emit` raises a bubbling, composed `CustomEvent` on the stepper's element, with the new value as its detail. The stepper has no idea who contains it; `main` listens on the body with `on`, which also hears events from anything inside. (`on` returns a `Listener`: it listens until it is dropped, so it too must be owned. `event_detail` reads the detail.) Events keep a component reusable: the same stepper works in a form, in a table, or in a page that listens from plain JavaScript.
- **Events are for what the user does.** `set_value` does not emit, so code that sets the value is not told about its own change.

## 2. A new element, with its own template and styles

When a component needs its own markup and CSS, give it its own element. A `Definition` describes the element type: its tag, the HTML of its shadow root, and a stylesheet. `create` makes an instance under a parent and returns the `Host`:

```rust
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
```

What to notice:

- **`Definition`** has five fields. `tag` is the element's name: it must contain a hyphen, and it must not be taken by something that is not moth (use a prefix of your own). `template` is the HTML of the shadow root, parsed once and cloned for each instance. `css` is one stylesheet shared by every instance (put long ones in a file and use `include_str!`). `attributes` names the attributes reported to a markup component (below), and `form_associated` is for elements that take part in a `<form>`.
- **`create`** registers the element type the first time it is used (using it again is fine) and fails if the tag belongs to something else. The `Host` it returns removes the element when dropped, which is why `Meter` just stores it.
- **`host.shadow("#fill")`** finds an element in your template, as the type you ask for (`HtmlElement` here; naming a type like that needs `web-sys` with that feature, see [getting started](getting-started.md#dependencies-you-may-need)). It fails with an error, not a panic, if nothing matches.
- **The inside is in a shadow root**, so the page's CSS does not reach it and your CSS does not leak out. Let the page restyle it through CSS variables with defaults (`var(--meter-fill, steelblue)`) and through `part="..."` attributes, which become `::part(fill)` selectors: [styling](styling.md) explains both. The stock components work exactly this way.
- **Not yet possible for a component of your own:** taking part in a form. `form_associated` is honoured, but the call that sets the value the form submits is not public yet; the stock `Edit`, `Slider` and `Dropdown` use it.

## 3. An element for markup

If a page is written in HTML (or generated by something else), your component can be an element in it. Implement `Declarative`: the same `Definition`, plus two functions. Here the page says `<guide-badge count="3">`:

```html
<p>You have <guide-badge id="unread" count="3"></guide-badge> unread messages.</p>
```

and the component:

```rust
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
```

What to notice:

- **The DOM owns the element here, and Rust owns the component.** `attach` runs when the element is connected to the page and builds the component around the `Host` moth hands it; `attribute_changed` runs for the attributes named in `attributes` (and once for each that is already present). The component lives as long as the element stays in the page: if the element is removed for good, the component is dropped, and when the tab goes away every markup component is dropped.
- **`define`** registers the element type; call it once at startup, with `define_all()` for the stock elements. Elements already in the page are upgraded.
- **Reaching it from Rust.** `Badge::of(&element)`, or `ElementRef::by_id("unread").unwrap().declared::<Badge>()`, gives a `Declared<Badge>`: a handle that works while the element exists and then reports that it is gone (`is_alive`), instead of dangling. `declared.with(|badge| badge.set_count(9))` runs a closure on the component.
- **The `disabled` attribute** is always observed and managed for you, like the `enabled` property: it is the same state.

## The rules

1. **Own everything you create**, in a field. A component you do not store is dropped, and its element removed, as soon as the function that made it returns.
2. **Callbacks capture handles** (`WeakProp`, `ElementRef`, `Rc<Cell<_>>` for plain data), never components or props that own things. The framework cannot stop you from capturing a component, but a closure that owns what owns the closure is a cycle that nothing frees.
3. **Use `Listener`, not raw closures,** for DOM events on elements you do not own: it removes the listener when dropped, and nothing is called afterwards.
4. **State goes in a `Prop`**, and components `bind` to it. `Prop::bind` lasts as long as the prop; a binding that should end sooner (a component shorter-lived than the prop it follows) uses `bind_scoped` and keeps the returned `Subscription`.
5. **Talk upward with events** (`emit`, `on`), downward with properties and methods.
6. **Test in a browser.** `moth/tests/guides.rs` shows how to build the components above, click them, and check what they do.
