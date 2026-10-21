//! Declarative components: elements written in markup (or created by any script) whose lifetime
//! the DOM controls. Rust owns the component, the DOM owns the element:
//!
//! - the component is created when its element first connects, with the attributes it already has;
//! - attribute changes are reported to it;
//! - it is dropped when its element is disconnected for good (moving an element, which
//!   disconnects and reconnects it within one task, keeps it), and on `pagehide`;
//! - an element connected again afterwards gets a fresh component, and the previous state is gone.

use std::{
	any::{Any, TypeId},
	cell::{Cell, RefCell},
	collections::HashMap,
	rc::Rc,
};

use js_sys::Reflect;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlElement;

use crate::{Component, Definition, Error, Host, component::{is_owned_by_rust, set_hook}};

/// A component backed by an element the DOM owns.
pub trait Declarative: Component + Sized + 'static {
	/// The custom element this component is built on: its tag, template, stylesheet and observed
	/// attributes.
	const DEFINITION: Definition;

	/// Build the component on `host`, which does not own its element. Its attributes are
	/// reported through [`Declarative::attribute_changed`] right after.
	fn attach(host: Host) -> Result<Self, Error>;

	/// An observed attribute (see [`Definition::attributes`]) changed; `None` means removed.
	/// Called once per observed attribute after `attach`, with its current value. `disabled` is
	/// handled by the framework (it is the host's own `enabled`).
	fn attribute_changed(&self, _name: &str, _value: Option<&str>) {}

	/// Typed, non-owning access to the component of an element written in markup (or created by a
	/// script): how Rust code reads and changes a component the page declared. `None` if the
	/// element has no component of this type, because it is not of this tag, it is not connected
	/// yet, or it is not a markup element (those are owned by Rust code that already has them).
	fn of(element: &HtmlElement) -> Option<Declared<Self>> {
		let id = instance_id(element)?;
		let instance = INSTANCES.with_borrow(|instances| instances.get(&id).cloned())?;
		let instance = instance.into_any().downcast::<Instance<Self>>().ok()?;
		Some(Declared { instance: Rc::downgrade(&instance) })
	}
}

/// Non-owning access to a markup element's component. The DOM owns the component's lifetime: once
/// the element is disconnected for good, [`Declared::with`] returns `None`.
pub struct Declared<T> {
	instance: std::rc::Weak<Instance<T>>,
}

impl<T> Clone for Declared<T> {
	fn clone(&self) -> Self {
		Self { instance: self.instance.clone() }
	}
}

impl<T: Declarative> Declared<T> {
	/// Run `f` on the component; `None` if it is gone. It stays alive while `f` runs, even if `f`
	/// removes its element.
	pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
		let instance = self.instance.upgrade()?;
		Some(f(&instance.0))
	}

	/// Whether the component still exists, that is, [`Declared::with`] would run its closure.
	pub fn is_alive(&self) -> bool {
		self.instance.strong_count() > 0
	}
}

/// Register a declarative component: elements with its tag, in the page already or added later,
/// get instantiated. A tag belongs to one component type; registering another type under it is an
/// error.
pub fn define<T: Declarative>() -> Result<(), Error> {
	let tag = T::DEFINITION.tag;
	let owner = TypeId::of::<T>();
	let taken = FACTORIES.with_borrow(|factories| {
		factories.get(tag).is_some_and(|(registered, _)| *registered != owner)
	});
	if taken {
		return Err(Error::new(format!("<{tag}> is already registered by another component type")));
	}

	let factory: Factory = Rc::new(|element: &HtmlElement| {
		let host = Host::new(element.clone(), false)?;
		let component = T::attach(host)?;
		for name in T::DEFINITION.attributes {
			component.attribute_changed(name, element.get_attribute(name).as_deref());
		}
		Ok(Rc::new(Instance(component)) as Rc<dyn Attached>)
	});
	// defining first: it fails if something foreign owns the tag, and must not leave a factory
	T::DEFINITION.define()?;
	FACTORIES.with_borrow_mut(|factories| factories.insert(tag, (owner, factory)));
	upgrade_existing(tag);
	Ok(())
}

/// Elements with this tag that were connected before it had a factory (the tag may have been
/// defined by a Rust constructor first) were ignored; give them their component now.
fn upgrade_existing(tag: &str) {
	let Some(document) = web_sys::window().and_then(|window| window.document()) else { return };
	let Ok(found) = document.query_selector_all(tag) else { return };
	for index in 0..found.length() {
		let element = found.item(index).and_then(|node| node.dyn_into::<HtmlElement>().ok());
		if let Some(element) = element.filter(|element| element.is_connected()) {
			connected(&element);
		}
	}
}

type Factory = Rc<dyn Fn(&HtmlElement) -> Result<Rc<dyn Attached>, Error>>;

trait Attached {
	fn attribute_changed(&self, name: &str, value: Option<&str>);
	/// To get back the concrete `Instance<T>` (see [`Declarative::of`]).
	fn into_any(self: Rc<Self>) -> Rc<dyn Any>;
}

struct Instance<T>(T);

impl<T: Declarative> Attached for Instance<T> {
	fn attribute_changed(&self, name: &str, value: Option<&str>) {
		// `disabled` is the host's business (it listens to the attribute for every element)
		if name != "disabled" {
			self.0.attribute_changed(name, value);
		}
	}

	fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
		self
	}
}

thread_local! {
	static FACTORIES: RefCell<HashMap<&'static str, (TypeId, Factory)>> = RefCell::new(HashMap::new());
	static INSTANCES: RefCell<HashMap<u32, Rc<dyn Attached>>> = RefCell::new(HashMap::new());
	static NEXT_ID: Cell<u32> = const { Cell::new(1) };
	static HOOKED: Cell<bool> = const { Cell::new(false) };
}

const ID_PROPERTY: &str = "mothId";
/// Stored in the id property while the factory runs: if building the component reconnects its own
/// element, the nested `connected` sees an id and does not build a second component.
const PENDING: f64 = 0.0;

/// Drop every markup-created component (the page is going away for good).
pub(crate) fn drop_all() {
	// destructors run outside any borrow of INSTANCES
	let all = INSTANCES.with_borrow_mut(std::mem::take);
	drop(all);
}

pub(crate) fn ensure_hook() {
	if HOOKED.replace(true) {
		return;
	}
	let hook = Closure::<dyn Fn(String, HtmlElement, Option<String>, Option<String>)>::new(
		|kind: String, element: HtmlElement, name: Option<String>, value: Option<String>| match kind
			.as_str()
		{
			"connected" => connected(&element),
			"disconnected" => disconnected(&element),
			"attribute" => {
				if let Some(name) = name {
					attribute(&element, &name, value.as_deref());
				}
			}
			_ => {}
		},
	);
	set_hook(hook.as_ref().unchecked_ref());
	// one process-wide registration: it lives as long as the page
	hook.forget();
}

fn instance_id(element: &HtmlElement) -> Option<u32> {
	Reflect::get(element, &ID_PROPERTY.into()).ok()?.as_f64().map(|id| id as u32)
}

fn connected(element: &HtmlElement) {
	if is_owned_by_rust(element) || instance_id(element).is_some() {
		return;
	}
	let factory = FACTORIES
		.with_borrow(|factories| factories.get(element.local_name().as_str()).map(|(_, f)| f.clone()));
	let Some(factory) = factory else { return };

	// a refused write only means the element is frozen; it then gets no instance
	if Reflect::set(element, &ID_PROPERTY.into(), &PENDING.into()).is_err() {
		return;
	}
	match factory(element) {
		Ok(instance) => {
			let id = NEXT_ID.replace(NEXT_ID.get() + 1);
			if Reflect::set(element, &ID_PROPERTY.into(), &f64::from(id).into()).is_ok() {
				INSTANCES.with_borrow_mut(|instances| instances.insert(id, instance));
			}
		}
		Err(error) => {
			let _ = Reflect::delete_property(element, &ID_PROPERTY.into());
			crate::console::error(&format!("<{}>: {error}", element.local_name()));
		}
	}
}

fn disconnected(element: &HtmlElement) {
	let Some(id) = instance_id(element) else { return };
	// the destructor runs here, outside any borrow of INSTANCES
	let removed = INSTANCES.with_borrow_mut(|instances| instances.remove(&id));
	let _ = Reflect::delete_property(element, &ID_PROPERTY.into());
	drop(removed);
}

fn attribute(element: &HtmlElement, name: &str, value: Option<&str>) {
	let Some(id) = instance_id(element) else { return };
	// cloned out so the callback may touch INSTANCES (e.g. by removing its own element)
	let instance = INSTANCES.with_borrow(|instances| instances.get(&id).cloned());
	if let Some(instance) = instance {
		instance.attribute_changed(name, value);
	}
}
