use std::{
	cell::{Cell, RefCell},
	rc::{Rc, Weak},
};

/// An observable value: the Rust-side half of moth's `DataPool`.
///
/// Setting the value pushes it to every binding (typically a DOM write); there is no diffing.
/// Setting a value equal to the current one does nothing, which is what keeps an attribute that
/// mirrors a prop from looping (prop -> attribute -> attributeChangedCallback -> prop...).
///
/// Re-entrancy is run-to-completion: a `set` issued from inside a binding (directly, or through
/// an event the binding provoked) stores its value at once but is *applied* to the bindings after
/// the running pass finishes. Bindings therefore receive values in order, and no borrow is held
/// while user code runs. (A `get()` made inside a binding already sees the newest stored value,
/// which can be newer than the one that binding was handed.)
///
/// A `Prop` is uniquely owned by its component. Callbacks must capture a [`WeakProp`], never the
/// owner.
///
/// [`Prop::bind`] bindings last as long as the prop; a component that is shorter-lived than the
/// prop it binds to must use [`Prop::bind_scoped`], or its closure (and what it captures) stays
/// in the prop and keeps running until the prop is dropped.
pub struct Prop<T> {
	inner: Rc<Inner<T>>,
}

struct Inner<T> {
	value: RefCell<T>,
	bindings: RefCell<Vec<(u64, Rc<dyn Fn(&T)>)>>,
	next_binding: Cell<u64>,
	applying: Cell<bool>,
	dirty: Cell<bool>,
}

impl<T: Clone + PartialEq + 'static> Inner<T> {
	fn set(&self, value: T) {
		let unchanged = *self.value.borrow() == value;
		if unchanged {
			return;
		}
		*self.value.borrow_mut() = value;
		self.flush();
	}

	fn flush(&self) {
		if self.applying.replace(true) {
			self.dirty.set(true);
			return;
		}
		loop {
			self.dirty.set(false);
			// bindings run on snapshots: no RefCell borrow is held while user code executes
			let value = self.value.borrow().clone();
			let bindings: Vec<Rc<dyn Fn(&T)>> =
				self.bindings.borrow().iter().map(|(_, binding)| binding.clone()).collect();
			for binding in &bindings {
				binding(&value);
			}
			if !self.dirty.get() {
				break;
			}
		}
		self.applying.set(false);
	}

	/// Register `binding` and apply the current value to it. Returns its id.
	fn bind(&self, binding: Rc<dyn Fn(&T)>) -> u64 {
		let id = self.next_binding.replace(self.next_binding.get() + 1);
		self.bindings.borrow_mut().push((id, binding.clone()));
		let value = self.value.borrow().clone();
		binding(&value);
		id
	}
}

impl<T: Clone + PartialEq + 'static> Prop<T> {
	/// A prop holding `value`, with no bindings.
	pub fn new(value: T) -> Self {
		Self {
			inner: Rc::new(Inner {
				value: RefCell::new(value),
				bindings: RefCell::new(Vec::new()),
				next_binding: Cell::new(0),
				applying: Cell::new(false),
				dirty: Cell::new(false),
			}),
		}
	}

	/// A copy of the current value.
	pub fn get(&self) -> T {
		self.inner.value.borrow().clone()
	}

	/// Store `value` and push it to every binding. Does nothing if it equals the current value; from
	/// inside a binding it is applied after the running pass (see [`Prop`]).
	pub fn set(&self, value: T) {
		self.inner.set(value);
	}

	/// Add a binding for the life of the prop and apply the current value to it immediately.
	pub fn bind(&self, binding: impl Fn(&T) + 'static) {
		self.inner.bind(Rc::new(binding));
	}

	/// Like [`Prop::bind`], but the binding is removed when the returned [`Subscription`] is
	/// dropped.
	#[must_use = "dropping the subscription removes the binding at once"]
	pub fn bind_scoped(&self, binding: impl Fn(&T) + 'static) -> Subscription {
		let id = self.inner.bind(Rc::new(binding));
		let inner = Rc::downgrade(&self.inner);
		Subscription {
			undo: Some(Box::new(move || {
				if let Some(inner) = inner.upgrade() {
					inner.bindings.borrow_mut().retain(|(bound, _)| *bound != id);
				}
			})),
		}
	}

	/// A non-owning [`WeakProp`] to this prop, for callbacks.
	pub fn downgrade(&self) -> WeakProp<T> {
		WeakProp { inner: Rc::downgrade(&self.inner) }
	}
}

/// Keeps a scoped binding alive; dropping it removes the binding (a no-op if the prop is gone).
pub struct Subscription {
	undo: Option<Box<dyn FnOnce()>>,
}

impl Drop for Subscription {
	fn drop(&mut self) {
		if let Some(undo) = self.undo.take() {
			undo();
		}
	}
}

/// Non-owning access to a [`Prop`]: safe to move into callbacks. Once the owning component is
/// dropped, `get` returns `None` and `set` returns `false` instead of touching anything.
pub struct WeakProp<T> {
	inner: Weak<Inner<T>>,
}

impl<T> Clone for WeakProp<T> {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone() }
	}
}

impl<T: Clone + PartialEq + 'static> WeakProp<T> {
	/// A copy of the current value, or `None` if the owning prop is gone.
	pub fn get(&self) -> Option<T> {
		self.inner.upgrade().map(|inner| inner.value.borrow().clone())
	}

	/// Returns whether the owner was still alive (and therefore the value applied).
	pub fn set(&self, value: T) -> bool {
		match self.inner.upgrade() {
			Some(inner) => {
				inner.set(value);
				true
			}
			None => false,
		}
	}

	/// Whether the owning prop still exists.
	pub fn is_alive(&self) -> bool {
		self.inner.strong_count() > 0
	}
}
