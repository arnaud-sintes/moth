use std::cell::RefCell;

use crate::{Prop, Subscription, WeakProp};

/// The client's copy of a model owned elsewhere (typically a server): the "empty shell" that
/// components are bound to.
///
/// The copy is never edited from the UI. User gestures become actions sent to the model's owner,
/// the owner's new model arrives through [`Replica::apply`], and bindings update the components.
/// Bindings select a field and run only when that field changed, so a large model does not
/// rewrite the whole page for each snapshot.
///
/// Divergence caveat: a control that changes itself on a gesture (a toggle flips, a text input
/// shows what was typed) shows the new value before the owner answers. If the owner then refuses
/// the action the model does not change, so no binding runs and the control keeps the refused
/// value.
pub struct Replica<M> {
	model: Prop<M>,
}

impl<M: Clone + PartialEq + 'static> Replica<M> {
	/// A replica holding `initial` until the first snapshot is applied.
	pub fn new(initial: M) -> Self {
		Self { model: Prop::new(initial) }
	}

	/// A copy of the current model.
	pub fn get(&self) -> M {
		self.model.get()
	}

	/// Replace the model with the owner's latest. Bindings whose field is unchanged do not run.
	pub fn apply(&self, model: M) {
		self.model.set(model);
	}

	/// Run `apply` with the value `select` extracts, now and whenever it changes.
	pub fn bind<T: Clone + PartialEq + 'static>(
		&self,
		select: impl Fn(&M) -> T + 'static,
		apply: impl Fn(&T) + 'static,
	) {
		let applied = RefCell::new(None::<T>);
		self.model.bind(move |model| {
			let value = select(model);
			let changed = applied.borrow().as_ref() != Some(&value);
			if changed {
				*applied.borrow_mut() = Some(value.clone());
				apply(&value);
			}
		});
	}

	/// Like [`Replica::bind`], but the binding is removed when the returned [`Subscription`] is
	/// dropped: for components that come and go while the replica lives on.
	#[must_use = "dropping the subscription removes the binding at once"]
	pub fn bind_scoped<T: Clone + PartialEq + 'static>(
		&self,
		select: impl Fn(&M) -> T + 'static,
		apply: impl Fn(&T) + 'static,
	) -> Subscription {
		let applied = RefCell::new(None::<T>);
		self.model.bind_scoped(move |model| {
			let value = select(model);
			let changed = applied.borrow().as_ref() != Some(&value);
			if changed {
				*applied.borrow_mut() = Some(value.clone());
				apply(&value);
			}
		})
	}

	/// Non-owning access, safe to move into callbacks (e.g. a socket's message handler).
	pub fn handle(&self) -> WeakReplica<M> {
		WeakReplica { model: self.model.downgrade() }
	}
}

/// Non-owning access to a [`Replica`]: once the replica is dropped, `apply` returns `false`.
pub struct WeakReplica<M> {
	model: WeakProp<M>,
}

impl<M> Clone for WeakReplica<M> {
	fn clone(&self) -> Self {
		Self { model: self.model.clone() }
	}
}

impl<M: Clone + PartialEq + 'static> WeakReplica<M> {
	/// A copy of the current model, or `None` if the replica is gone.
	pub fn get(&self) -> Option<M> {
		self.model.get()
	}

	/// Returns whether the replica was still alive.
	pub fn apply(&self, model: M) -> bool {
		self.model.set(model)
	}
}
