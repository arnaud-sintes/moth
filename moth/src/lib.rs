//! moth: parent/child components over Web Components, with Rust `Drop` as the destructor.
//!
//! Rust owns every component; the DOM is only a view. Dropping a component removes its element
//! and unregisters its listeners. Components hold node references (never string ids), callbacks
//! own their closures and capture non-owning handles, so no drop order can leave anything
//! dangling.
//!
//! Elements written in markup are supported too ([`Declarative`]): there the DOM owns the
//! element and Rust owns the component, which lives as long as the element stays connected.
//!
//! # A first page
//!
//! A project is a binary crate built to WebAssembly (with [trunk]), an `index.html`, and a `main`.
//! Call [`init`] first, create a [`Body`], create components under it, and hand the page to
//! [`mount`] so that it lives until the browser tab goes away:
//!
//! ```no_run
//! use moth::{Body, Label, PushButton};
//!
//! struct Page {
//!     _body: Body,
//!     _label: Label,
//!     _button: PushButton,
//! }
//!
//! fn main() {
//!     moth::init();
//!     let body = Body::new().unwrap();
//!     let label = Label::new(&body, "Hello world").unwrap();
//!     // callbacks capture a non-owning handle, never the components themselves
//!     let text = label.text_handle();
//!     let button = PushButton::new(&body, "Reset", move || {
//!         text.set("Hello again".to_owned());
//!     })
//!     .unwrap();
//!     moth::mount(Page { _body: body, _label: label, _button: button }).unwrap();
//! }
//! ```
//!
//! # Where to look
//!
//! - **Components**: [`Body`], [`Panel`], [`Label`], [`PushButton`], [`ToggleButton`], [`Image`],
//!   [`Edit`], [`Slider`], [`Dropdown`], [`ProgressBar`], [`Canvas`]. Anything that implements
//!   [`Component`] can be the parent of others (see [`Parent`]); your own components are made by
//!   composing these, or from a [`Definition`].
//! - **State**: [`Prop`] holds a value and writes it to whatever is bound to it, [`WeakProp`] is
//!   the handle callbacks keep. [`Keyed`] and [`Children`] keep components in step with a list.
//! - **Events**: [`Component::emit`] and [`Component::on`] (composed, bubbling custom events),
//!   [`Listener`] (a DOM listener that ends when dropped), [`Timeout`].
//! - **Markup**: [`define_all`] and [`Declarative`] make the components usable in HTML.
//! - **The network**: [`WebSocket`]; with the `replication` feature [`Link`], [`Replica`] and
//!   [`Actions`] keep a model that lives on a server in step with the page.
//!
//! The guides in the repository's `docs/` folder (getting started, a tutorial, writing components,
//! styling) go further than this reference.
//!
//! moth targets Chromium (Chrome, Edge) only.
//!
//! [trunk]: https://trunk-rs.github.io/trunk/

#![warn(missing_docs)]

mod children;
mod component;
pub mod console;
mod declarative;
mod elements;
mod error;
mod keyed;
mod listener;
mod prop;
mod replica;
mod timeout;
mod websocket;

#[cfg(feature = "replication")]
mod link;

use std::{any::Any, cell::RefCell};

use wasm_bindgen::JsCast;

pub use children::{Children, Key};
/// Names of the events the stock components raise (see [`Component::emit`]). Each bubbles and is
/// composed, so a parent in Rust or a script in the page can listen for it.
pub mod events {
	/// A [`crate::ToggleButton`] was clicked while enabled. `detail`: the state after the click,
	/// or, for a controlled toggle, the state it asks for (a `bool`).
	pub const TOGGLE: &str = "moth-toggle";
	/// The user picked an option of a [`crate::Dropdown`]. `detail`: the index, or `null` for none.
	/// (The native `change` event is not composed, so a page could not hear it.)
	pub const SELECT: &str = "moth-select";
}

pub use component::{Anchor, Component, Definition, ElementRef, Host, Parent, emit_from, event_detail};
pub use declarative::{Declarative, Declared, define};
pub use elements::{
	Body, Canvas, CanvasHandle, Dropdown, Edit, Image, Label, Panel, ProgressBar, ProgressHandle,
	PushButton, Size, Slider, SliderHandle, ToggleButton,
};
pub use error::Error;
pub use keyed::Keyed;
pub use listener::Listener;
pub use prop::{Prop, Subscription, WeakProp};
pub use replica::{Replica, WeakReplica};
pub use timeout::Timeout;
pub use websocket::{Close, Handlers, Message, SocketHandle, State, WebSocket};

#[cfg(feature = "replication")]
pub use link::{Actions, Backoff, Link, LinkEvent, LinkOptions};

thread_local! {
	static ROOTS: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
	static PAGE_HIDE: RefCell<Option<Listener>> = const { RefCell::new(None) };
}

/// What every moth application wants done first: panics are written to the browser console, with
/// their file and line, instead of ending as an opaque `unreachable` error (WebAssembly cannot unwind,
/// so a panic stops the program; this is how you learn why). Call it at the top of `main`; calling it
/// more than once does nothing more.
pub fn init() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		std::panic::set_hook(Box::new(|info| console::error(&info.to_string())));
	});
}

/// Make every stock element usable in markup: `<moth-panel>`, `<moth-label>`,
/// `<moth-push-button>`, `<moth-toggle-button>`, `<moth-image>`, `<moth-progress-bar>`,
/// `<moth-edit>`, `<moth-slider>`. Call it once at startup; elements already in the page are
/// upgraded.
pub fn define_all() -> Result<(), Error> {
	define::<Panel>()?;
	define::<Label>()?;
	define::<PushButton>()?;
	define::<ToggleButton>()?;
	define::<Image>()?;
	define::<ProgressBar>()?;
	define::<Edit>()?;
	define::<Slider>()
}

/// Keep a root component alive for the lifetime of the page, and drop it on `pagehide`
/// (the only reliable teardown hook: destructors do not run on their own when a tab closes).
///
/// Teardown also destroys every markup-created component. A `pagehide` with `persisted` set means
/// the browser keeps the page in its back/forward cache and may show it again, so nothing is
/// destroyed then; a restored page needs its connections re-established, which is the
/// application's business (sockets do not survive the cache).
pub fn mount(root: impl Any) -> Result<(), Error> {
	ROOTS.with_borrow_mut(|roots| roots.push(Box::new(root)));
	if PAGE_HIDE.with_borrow(Option::is_some) {
		return Ok(());
	}
	let window = web_sys::window().ok_or_else(|| Error::new("no window"))?;
	let listener = Listener::new(window.unchecked_ref(), "pagehide", |event| {
		let may_come_back = event
			.dyn_ref::<web_sys::PageTransitionEvent>()
			.is_some_and(web_sys::PageTransitionEvent::persisted);
		if may_come_back {
			return;
		}
		// take the roots out first: their destructors must not run while ROOTS is borrowed
		let roots = ROOTS.with_borrow_mut(std::mem::take);
		drop(roots);
		declarative::drop_all();
	})?;
	PAGE_HIDE.with_borrow_mut(|slot| *slot = Some(listener));
	Ok(())
}
