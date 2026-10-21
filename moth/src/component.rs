use js_sys::Reflect;
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{CustomEvent, CustomEventInit, Element, Event, EventTarget, HtmlElement};

use crate::{Error, Listener, Prop, WeakProp, declarative};

#[wasm_bindgen(module = "/src/shim.js")]
unsafe extern "C" {
	#[wasm_bindgen(catch)]
	fn define_element(
		tag: &str,
		template: &str,
		css: &str,
		attributes: Vec<String>,
		form_associated: bool,
	) -> Result<(), JsValue>;

	pub(crate) fn set_hook(callback: &js_sys::Function);

	#[wasm_bindgen(catch)]
	fn set_form_value(element: &HtmlElement, value: Option<String>) -> Result<(), JsValue>;
}

/// Marks elements created by Rust, as opposed to elements the HTML parser (or any other script)
/// created: only the latter are instantiated by the declarative machinery.
const OWNED_BY_RUST: &str = "mothOwned";

/// Dispatched on a component's element when its ancestors may have changed (it connected), and on
/// every light-DOM descendant when its own `enabled` changed.
const ENABLED_CHANGED: &str = "moth-enabled-changed";

/// Dispatched (by the shim) on a component's element when its `disabled` attribute changed.
const DISABLED_ATTRIBUTE: &str = "moth-disabled-attribute";

/// Dispatch a `CustomEvent` named `name` carrying `detail` from `target`. It bubbles and is
/// *composed*, so it also crosses shadow-root boundaries: an element inside a component's shadow
/// root can raise it and every ancestor, in Rust or in markup, can listen with
/// `addEventListener`. Listeners run before this returns.
pub fn emit_from(target: &EventTarget, name: &str, detail: &JsValue) -> Result<(), Error> {
	let init = CustomEventInit::new();
	init.set_bubbles(true);
	init.set_composed(true);
	init.set_detail(detail);
	let event = CustomEvent::new_with_event_init_dict(name, &init)?;
	target.dispatch_event(&event)?;
	Ok(())
}

/// The `detail` of a `CustomEvent`; `None` for any other event.
pub fn event_detail(event: &Event) -> Option<JsValue> {
	event.dyn_ref::<CustomEvent>().map(CustomEvent::detail)
}

/// Something a component can be created under: any [`Component`], or an [`ElementRef`] over an
/// element nobody wraps.
pub trait Parent {
	/// The element new components are appended to.
	fn parent_element(&self) -> &HtmlElement;
}

impl<T: Component + ?Sized> Parent for T {
	fn parent_element(&self) -> &HtmlElement {
		self.element()
	}
}

/// Anything that owns a DOM host element and can therefore be a parent.
pub trait Component {
	/// The [`Host`] the component holds; every other method is provided through it.
	fn host(&self) -> &Host;

	/// The component's DOM element.
	fn element(&self) -> &HtmlElement {
		self.host().element()
	}

	/// The component's own `enabled` property (set with [`Component::set_enabled`]).
	fn enabled(&self) -> bool {
		self.host().enabled.get()
	}

	/// Set the component's own `enabled` property, which writes the `disabled` attribute of its
	/// element. Setting the value it already has does nothing.
	fn set_enabled(&self, enabled: bool) {
		self.host().enabled.set(enabled);
	}

	/// Whether the component can really be used: enabled itself and under no disabled ancestor.
	/// Read from the DOM each time, so it is right even when a non-moth ancestor (a
	/// `<fieldset>`) changed and nothing could tell the component.
	fn effectively_enabled(&self) -> bool {
		effectively_enabled_now(self.element())
	}

	/// Non-owning access to the own `enabled` property, for callbacks.
	fn enabled_handle(&self) -> WeakProp<bool> {
		self.host().enabled.downgrade()
	}

	/// Tell the ancestors (and the page) something happened here: a bubbling, composed
	/// `CustomEvent` on this component's element, see [`emit_from`]. The way a child talks to
	/// whoever contains it without knowing who that is.
	fn emit(&self, name: &str, detail: &JsValue) -> Result<(), Error> {
		emit_from(self.element(), name, detail)
	}

	/// Listen on this component's element: to events it raises itself and to those bubbling up from
	/// anything inside it, such as its children's `emit`s. Stops when the returned [`Listener`] is
	/// dropped.
	fn on(&self, name: &'static str, handler: impl FnMut(Event) + 'static) -> Result<Listener, Error>
	where
		Self: Sized,
	{
		Listener::new(self.element(), name, handler)
	}
}

/// A non-owning, inert reference to an existing element, to create components under it. Unlike
/// [`Anchor`] it adds nothing to the element (no properties, no listeners), so any number of
/// them can coexist with the element's own component.
#[derive(Clone)]
pub struct ElementRef {
	element: HtmlElement,
}

impl ElementRef {
	/// A reference to `element`; the element is cloned (a JS reference) and left untouched.
	pub fn new(element: &HtmlElement) -> Self {
		Self { element: element.clone() }
	}

	/// The referenced element.
	pub fn element(&self) -> &HtmlElement {
		&self.element
	}

	/// The element with this id, if the page has one.
	pub fn by_id(id: &str) -> Option<Self> {
		web_sys::window()?
			.document()?
			.get_element_by_id(id)?
			.dyn_into::<HtmlElement>()
			.ok()
			.map(|element| Self { element })
	}

	/// Run `handler` for each `name` event on the element (or bubbling up from inside it) for as
	/// long as the returned [`Listener`] lives: behaviour for an element that is already in the
	/// page, owned by Rust code, gone when that code drops it. The element is left as it is.
	/// Not guarded by the element's enabled state; see [`ElementRef::effectively_enabled`].
	pub fn on(&self, name: &'static str, handler: impl FnMut(Event) + 'static) -> Result<Listener, Error> {
		Listener::new(&self.element, name, handler)
	}

	/// See [`emit_from`].
	pub fn emit(&self, name: &str, detail: &JsValue) -> Result<(), Error> {
		emit_from(&self.element, name, detail)
	}

	/// Whether the element and all its ancestors are enabled (no `disabled` attribute), read now.
	pub fn effectively_enabled(&self) -> bool {
		effectively_enabled_now(&self.element)
	}

	/// The typed component that markup-created `<moth-...>` element carries, if it has one of type
	/// `T` (see [`crate::Declarative::of`]).
	pub fn declared<T: crate::Declarative>(&self) -> Option<crate::Declared<T>> {
		T::of(&self.element)
	}
}

impl Parent for ElementRef {
	fn parent_element(&self) -> &HtmlElement {
		&self.element
	}
}

/// Static description of a custom element type: tag, shadow template and shared stylesheet.
pub struct Definition {
	/// Tag name of the custom element, for example `moth-label`.
	pub tag: &'static str,
	/// HTML of the shadow root, parsed once per tag and cloned for each instance.
	pub template: &'static str,
	/// Stylesheet shared (as a constructable stylesheet) by every instance's shadow root.
	pub css: &'static str,
	/// Attributes reported to a declarative component (`disabled` is always observed).
	pub attributes: &'static [&'static str],
	/// Participates in forms: the element's value is submitted with its `<form>`.
	pub form_associated: bool,
}

impl Definition {
	/// Register the custom element (idempotent) and the lifecycle hook. Fails if something that is
	/// not moth already defined the tag.
	pub(crate) fn define(&self) -> Result<(), Error> {
		declarative::ensure_hook();
		let mut attributes: Vec<String> = self.attributes.iter().map(|name| (*name).into()).collect();
		attributes.push("disabled".into());
		define_element(self.tag, self.template, self.css, attributes, self.form_associated)?;
		Ok(())
	}

	/// Create an instance owned by Rust, attached under `parent` (or `<body>` when there is none):
	/// the way a component with its own template and stylesheet is built from Rust. The element type
	/// is registered on first use. The returned [`Host`] removes the element when dropped, so a
	/// component that holds it (and implements [`Component`]) cleans up like the stock ones.
	///
	/// Fails if the tag was defined by something that is not moth.
	pub fn create(&self, parent: Option<&HtmlElement>) -> Result<Host, Error> {
		self.define()?;
		let document = web_sys::window()
			.and_then(|window| window.document())
			.ok_or_else(|| Error::new("no document"))?;
		let element: HtmlElement = document
			.create_element(self.tag)?
			.dyn_into()
			.map_err(|_| Error::new(format!("<{}> is not an HtmlElement", self.tag)))?;
		Reflect::set(&element, &OWNED_BY_RUST.into(), &true.into())?;
		let parent = match parent {
			Some(parent) => parent.clone(),
			None => document.body().ok_or_else(|| Error::new("no body"))?,
		};
		parent.append_child(&element)?;
		Host::new(element, true)
	}
}

pub(crate) fn is_owned_by_rust(element: &HtmlElement) -> bool {
	Reflect::get(element, &OWNED_BY_RUST.into()).is_ok_and(|value| value.is_truthy())
}

/// A component's DOM host element, with the properties every element has.
///
/// Holds a node reference rather than an id, and `remove()` is idempotent, so teardown is safe
/// whatever order a parent and its children are dropped in. A host created by Rust removes its
/// element on drop; a host adopted from an element the DOM owns (see [`Anchor`] and
/// [`crate::Declarative`]) leaves it alone.
///
/// `enabled` is the component's own state and *is* the `disabled` attribute of the host
/// (`<moth-x disabled>`): writing the attribute from anywhere (markup, a script) changes it, and
/// changing it writes the attribute. It is read from the attribute when an existing element is
/// adopted. What the inner controls follow is the effective state: this and every ancestor.
pub struct Host {
	element: HtmlElement,
	owns_element: bool,
	pub(crate) enabled: Prop<bool>,
	pub(crate) effective: Prop<bool>,
	_inherit: Listener,
	_attribute: Listener,
}

impl Host {
	pub(crate) fn new(element: HtmlElement, owns_element: bool) -> Result<Self, Error> {
		let enabled = Prop::new(!element.has_attribute("disabled"));
		let effective = Prop::new(effectively_enabled_now(&element));

		// the ancestors may have changed (an ancestor toggled, or this element was moved): recompute
		// (the ancestor walk reads attributes, so one event per descendant is enough, there is no
		// need to forward it level by level)
		let (own, derived, target) = (enabled.downgrade(), effective.downgrade(), element.clone());
		let inherit = Listener::new(&element, ENABLED_CHANGED, move |_| {
			derived.set(own.get() == Some(true) && !ancestor_disabled(&target));
		})?;

		// the attribute changed from outside: it is the own state
		let (own, target) = (enabled.downgrade(), element.clone());
		let attribute = Listener::new(&element, DISABLED_ATTRIBUTE, move |_| {
			own.set(!target.has_attribute("disabled"));
		})?;

		let (derived, target) = (effective.downgrade(), element.clone());
		enabled.bind(move |enabled| {
			set_flag(&target, "disabled", !enabled);
			derived.set(*enabled && !ancestor_disabled(&target));
			notify_descendants(&target);
		});
		Ok(Self { element, owns_element, enabled, effective, _inherit: inherit, _attribute: attribute })
	}

	/// The host element.
	pub fn element(&self) -> &HtmlElement {
		&self.element
	}

	/// The first element matching `selector` in the shadow root (the component's template), as `T`
	/// (for example [`web_sys::HtmlInputElement`]). Errors if the element has no shadow root, nothing
	/// matches, or the match is not a `T`.
	pub fn shadow<T: JsCast>(&self, selector: &str) -> Result<T, Error> {
		let root = self
			.element
			.shadow_root()
			.ok_or_else(|| Error::new("element has no shadow root"))?;
		root.query_selector(selector)?
			.ok_or_else(|| Error::new(format!("no `{selector}` in the shadow template")))?
			.dyn_into()
			.map_err(|_| Error::new(format!("`{selector}` has an unexpected type")))
	}
}

impl Component for Host {
	fn host(&self) -> &Host {
		self
	}
}

impl Drop for Host {
	fn drop(&mut self) {
		if self.owns_element {
			self.element.remove();
		}
	}
}

/// Submit `value` with the enclosing form (`None`: nothing). The element must have been defined
/// with `form_associated`.
pub(crate) fn set_form_value_on(element: &HtmlElement, value: Option<String>) {
	// failing only if the element is not form-associated, which is a programming error of the
	// component definition and shows up in its first test
	let _ = set_form_value(element, value);
}

/// A non-owning component over an element that already exists, to use as the root of a Rust
/// component tree inside a page that was not built by moth (`#app`, a `<form>`...), with the
/// `enabled` property every component has. Dropping it leaves the element in place.
///
/// To merely create components under an element, an [`ElementRef`] is enough; do not wrap an
/// element that already has a component: both would own its `disabled` state.
pub struct Anchor {
	host: Host,
}

impl Anchor {
	/// Wrap `element`, reading its `enabled` state from its `disabled` attribute. The element is
	/// never removed by this component. Fails if the listeners cannot be attached.
	pub fn new(element: &HtmlElement) -> Result<Self, Error> {
		Ok(Self { host: Host::new(element.clone(), false)? })
	}

	/// The element with this id, if the page has one.
	pub fn by_id(id: &str) -> Result<Option<Self>, Error> {
		let element = web_sys::window()
			.and_then(|window| window.document())
			.and_then(|document| document.get_element_by_id(id))
			.and_then(|element| element.dyn_into::<HtmlElement>().ok());
		element.map(|element| Self::new(&element)).transpose()
	}
}

impl Component for Anchor {
	fn host(&self) -> &Host {
		&self.host
	}
}

fn ancestor_disabled(element: &HtmlElement) -> bool {
	// `closest` includes the element itself, hence starting at the parent. A plain
	// `<fieldset disabled>` ancestor disables moth components too, like native controls.
	element
		.parent_element()
		.and_then(|parent| parent.closest("[disabled]").ok().flatten())
		.is_some()
}

/// The element and every ancestor are enabled, read from the DOM right now. What handlers check:
/// unlike the `effective` property it cannot be stale.
pub(crate) fn effectively_enabled_now(element: &HtmlElement) -> bool {
	!element.has_attribute("disabled") && !ancestor_disabled(element)
}

fn notify_descendants(element: &HtmlElement) {
	let Ok(descendants) = element.query_selector_all("*") else { return };
	for index in 0..descendants.length() {
		let Some(descendant) = descendants.item(index) else { continue };
		if let (Ok(target), Ok(event)) =
			(descendant.dyn_into::<web_sys::EventTarget>(), Event::new(ENABLED_CHANGED))
		{
			let _ = target.dispatch_event(&event);
		}
	}
}

/// Reflect a boolean as a presence attribute.
pub(crate) fn set_flag(element: &Element, name: &str, on: bool) {
	// attribute names are compile-time constants, so a failure here is not actionable
	let _ = if on { element.set_attribute(name, "") } else { element.remove_attribute(name) };
}

/// Keep the text of `element` in sync with `text`.
///
/// `apply_initial: true` (an element made by Rust) keeps the text in a text node of its own, first
/// among the children, so that child components added to the element are never touched by a text
/// change. `false` (an element written in markup) leaves the content alone until the first change,
/// which then replaces all of it: in markup the content is the text.
pub(crate) fn bind_text(text: &Prop<String>, element: &HtmlElement, apply_initial: bool) {
	if apply_initial {
		let node = element.owner_document().map(|document| document.create_text_node(""));
		if let Some(node) = node {
			if element.insert_before(&node, element.first_child().as_ref()).is_ok() {
				text.bind(move |text| node.set_text_content(Some(text)));
				return;
			}
		}
	}
	let element = element.clone();
	let skip = std::cell::Cell::new(!apply_initial);
	text.bind(move |text| {
		if !skip.replace(false) {
			element.set_text_content(Some(text));
		}
	});
}

/// A number as HTML parses one (a "valid floating-point number"): digits with an optional sign,
/// fraction and exponent, finite. Rust's own parser also takes `inf`, `NaN` and a leading `+`,
/// which a browser's `<input>` does not, so a value accepted here must be one the inner control
/// accepts too.
pub(crate) fn parse_html_number(text: &str) -> Option<f64> {
	let text = text.trim();
	let plausible = !text.starts_with('+')
		&& text.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E'));
	if !plausible {
		return None;
	}
	text.parse::<f64>().ok().filter(|number| number.is_finite())
}
