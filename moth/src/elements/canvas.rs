use std::{
	cell::{Cell, RefCell},
	rc::{Rc, Weak},
};

use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
	CanvasRenderingContext2d, Event, HtmlCanvasElement, HtmlElement, MediaQueryList, ResizeObserver,
};

use crate::{Component, Definition, Error, Host, Parent};

/// Drawing area handed to the draw callback, in CSS pixels. The context is already scaled by
/// `scale` (the device pixel ratio), so the callback draws in CSS pixels and stays sharp on
/// high-density screens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
	/// Width of the drawing area, in CSS pixels.
	pub width: f64,
	/// Height of the drawing area, in CSS pixels.
	pub height: f64,
	/// The device pixel ratio the context is scaled by.
	pub scale: f64,
}

type Draw = Box<dyn FnMut(&CanvasRenderingContext2d, Size)>;

/// Fires when the device pixel ratio stops being the one it was created for.
struct PixelRatioWatch {
	query: MediaQueryList,
	on_change: Closure<dyn FnMut(Event)>,
}

impl Drop for PixelRatioWatch {
	fn drop(&mut self) {
		let _ = self
			.query
			.remove_event_listener_with_callback("change", self.on_change.as_ref().unchecked_ref());
	}
}

struct Shared {
	me: Weak<Shared>,
	host: HtmlElement,
	canvas: HtmlCanvasElement,
	context: CanvasRenderingContext2d,
	on_draw: RefCell<Draw>,
	/// Pending `requestAnimationFrame` id: set while a redraw is scheduled, which is what
	/// coalesces any number of `invalidate` calls into one draw per frame.
	frame: Cell<Option<i32>>,
	/// The animation frame callback. It holds a `Weak<Shared>`; `Shared` holds it: dropping the
	/// `Canvas` takes it out, which ends the cycle.
	paint: RefCell<Option<Closure<dyn FnMut(f64)>>>,
	pixel_ratio: RefCell<Option<PixelRatioWatch>>,
}

impl Shared {
	fn invalidate(&self) {
		if self.frame.get().is_some() {
			return;
		}
		let (Some(window), paint) = (web_sys::window(), self.paint.borrow()) else { return };
		let Some(paint) = paint.as_ref() else { return };
		if let Ok(id) = window.request_animation_frame(paint.as_ref().unchecked_ref()) {
			self.frame.set(Some(id));
		}
	}

	fn cancel(&self) {
		if let (Some(id), Some(window)) = (self.frame.take(), web_sys::window()) {
			window.cancel_animation_frame(id).ok();
		}
	}

	/// Match the backing store to the displayed size and pixel ratio. Resizing clears the canvas,
	/// so a redraw is always requested afterwards.
	fn resize(&self) {
		let scale = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
		let (width, height) = (f64::from(self.host.client_width()), f64::from(self.host.client_height()));
		self.canvas.set_width((width * scale).round() as u32);
		self.canvas.set_height((height * scale).round() as u32);
		self.invalidate();
	}

	/// Be told when the pixel ratio changes without the element's CSS size changing (zoom, a window
	/// moved to another monitor): the `ResizeObserver` does not see that. A `(resolution: Xdppx)`
	/// query changes when the ratio leaves X, so it is re-created for each new ratio.
	fn watch_pixel_ratio(&self) {
		let Some(window) = web_sys::window() else { return };
		let query = window
			.match_media(&format!("(resolution: {}dppx)", window.device_pixel_ratio()))
			.ok()
			.flatten();
		let Some(query) = query else { return };
		let weak = self.me.clone();
		let on_change = Closure::<dyn FnMut(Event)>::new(move |_| {
			if let Some(shared) = weak.upgrade() {
				shared.resize();
				shared.watch_pixel_ratio();
			}
		});
		if query.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref()).is_ok() {
			// replacing the watch that is calling us is fine: freeing a closure that is running is
			// deferred until it returns
			let previous = self.pixel_ratio.borrow_mut().replace(PixelRatioWatch { query, on_change });
			drop(previous);
		}
	}

	fn paint(&self) {
		// cleared first so the callback may ask for the next frame (an animation loop)
		self.frame.set(None);
		let scale = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
		let size = Size {
			width: f64::from(self.canvas.width()) / scale,
			height: f64::from(self.canvas.height()) / scale,
			scale,
		};
		let context = &self.context;
		// failing only on a non-finite matrix, which `scale` cannot be
		let _ = context.set_transform(scale, 0.0, 0.0, scale, 0.0, 0.0);
		context.clear_rect(0.0, 0.0, size.width, size.height);
		(self.on_draw.borrow_mut())(context, size);
	}
}

/// A `<canvas>` that stays sharp and up to date by itself:
///
/// - its backing store follows the element's CSS size and the device pixel ratio (observed with
///   a `ResizeObserver` and a resolution media query), and the draw callback runs again after each
///   change;
/// - redraws are requested with [`Canvas::invalidate`] and coalesced to one per animation frame;
/// - dropping it cancels the pending frame and disconnects the observers, so the callback never
///   runs after the drop.
///
/// Size it with CSS (`moth-canvas { height: 300px }`, default `--moth-canvas-height: 150px`).
pub struct Canvas {
	shared: Rc<Shared>,
	observer: ResizeObserver,
	_on_resize: Closure<dyn FnMut(js_sys::Array, ResizeObserver)>,
	host: Host,
}

impl Canvas {
	const DEF: Definition = Definition {
		tag: "moth-canvas",
		template: include_str!("canvas.html"),
		css: include_str!("canvas.css"),
		attributes: &[],
		form_associated: false,
	};

	/// Create a canvas under `parent`. `on_draw` receives the 2d context and the [`Size`]; the
	/// canvas is cleared before each call, and the call happens on an animation frame requested by
	/// [`Canvas::invalidate`] or by a change of size or pixel ratio, never after the canvas is
	/// dropped. Fails if the 2d context is unavailable.
	pub fn new(
		parent: &impl Parent,
		on_draw: impl FnMut(&CanvasRenderingContext2d, Size) + 'static,
	) -> Result<Self, Error> {
		let host = Self::DEF.create(Some(parent.parent_element()))?;
		let canvas: HtmlCanvasElement = host.shadow("canvas")?;
		let context = canvas
			.get_context("2d")?
			.ok_or_else(|| Error::new("2d canvas context unavailable"))?
			.dyn_into::<CanvasRenderingContext2d>()
			.map_err(|_| Error::new("unexpected canvas context type"))?;

		let shared = Rc::new_cyclic(|me| Shared {
			me: me.clone(),
			host: host.element().clone(),
			canvas,
			context,
			on_draw: RefCell::new(Box::new(on_draw)),
			frame: Cell::new(None),
			paint: RefCell::new(None),
			pixel_ratio: RefCell::new(None),
		});
		let weak = Rc::downgrade(&shared);
		*shared.paint.borrow_mut() = Some(Closure::<dyn FnMut(f64)>::new(move |_| {
			if let Some(shared) = weak.upgrade() {
				shared.paint();
			}
		}));

		let weak = Rc::downgrade(&shared);
		let on_resize = Closure::<dyn FnMut(js_sys::Array, ResizeObserver)>::new(
			move |_entries, _observer| {
				if let Some(shared) = weak.upgrade() {
					shared.resize();
				}
			},
		);
		let observer = ResizeObserver::new(on_resize.as_ref().unchecked_ref())?;
		observer.observe(host.element());
		shared.watch_pixel_ratio();
		Ok(Self { shared, observer, _on_resize: on_resize, host })
	}

	/// Request a redraw on the next animation frame; any number of calls before it count once.
	pub fn invalidate(&self) {
		self.shared.invalidate();
	}

	/// Non-owning access for callbacks (e.g. a model binding).
	pub fn handle(&self) -> CanvasHandle {
		CanvasHandle { shared: Rc::downgrade(&self.shared) }
	}
}

impl Component for Canvas {
	fn host(&self) -> &Host {
		&self.host
	}
}

impl Drop for Canvas {
	fn drop(&mut self) {
		self.observer.disconnect();
		self.shared.cancel();
		let watch = self.shared.pixel_ratio.borrow_mut().take();
		drop(watch);
		// ends the Rc cycle; also what makes a drop from inside the draw callback safe, since
		// `Shared` itself stays alive until that call returns
		self.shared.paint.borrow_mut().take();
	}
}

/// Non-owning access to a [`Canvas`]; safe to keep in callbacks.
#[derive(Clone)]
pub struct CanvasHandle {
	shared: Weak<Shared>,
}

impl CanvasHandle {
	/// Returns whether the canvas was still alive.
	pub fn invalidate(&self) -> bool {
		match self.shared.upgrade() {
			Some(shared) => {
				shared.invalidate();
				true
			}
			None => false,
		}
	}
}
