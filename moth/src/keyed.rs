use std::{collections::{HashMap, HashSet}, hash::Hash};

use web_sys::{HtmlElement, Node};

use crate::{Component, Error};

/// Components that mirror a list of model items, matched by a stable key.
///
/// [`Keyed::sync`] makes the components match the items: a component whose key is gone is dropped
/// (so its element and listeners go with it), a new key gets a component, an existing key keeps its
/// component, and the elements end up in the items' order. Nothing is rebuilt for an item that
/// merely changed, and only the elements that must move are moved (moving an element loses its
/// focus and selection, so a list with inputs would otherwise lose them on every reorder).
///
/// The rows form one contiguous block of the parent, ending at a marker node (an empty comment)
/// that the `Keyed` puts at the end of the parent on its first sync, or before a given node
/// ([`Keyed::before`]). Other children of the parent are left alone: those before the block stay
/// before, those after the marker stay after. Use one parent per `Keyed`, and do not reorder its
/// elements by hand (the `Keyed` remembers the order it made).
pub struct Keyed<K, T> {
	entries: HashMap<K, T>,
	/// The keys in the order their elements are in the DOM, as of the last sync.
	order: Vec<K>,
	marker: Option<Node>,
	before: Option<Node>,
}

impl<K, T> Default for Keyed<K, T> {
	fn default() -> Self {
		Self { entries: HashMap::new(), order: Vec::new(), marker: None, before: None }
	}
}

impl<K, T> Drop for Keyed<K, T> {
	fn drop(&mut self) {
		if let Some(marker) = self.marker.take() {
			if let Some(parent) = marker.parent_node() {
				let _ = parent.remove_child(&marker);
			}
		}
	}
}

impl<K: Hash + Eq + Clone, T: Component> Keyed<K, T> {
	/// The rows go at the end of the parent.
	pub fn new() -> Self {
		Self::default()
	}

	/// The rows go before `node`, a child of the parent that will be passed to `sync`.
	pub fn before(node: &Node) -> Self {
		Self {
			entries: HashMap::new(),
			order: Vec::new(),
			marker: None,
			before: Some(node.clone()),
		}
	}

	/// - `key` identifies an item; keys must be unique within `items`, or nothing is changed and
	///   an error is returned;
	/// - `create` builds the component of a new item (under the same parent as `parent`);
	/// - `update` brings an existing component up to date with its item, called for every item
	///   that already had one;
	/// - `parent` is the element the components live in.
	///
	/// Not atomic: the components of vanished keys are dropped first, and if `create` fails the
	/// components made so far stay (the next sync puts them in place) and the error is returned.
	pub fn sync<I>(
		&mut self,
		parent: &HtmlElement,
		items: &[I],
		key: impl Fn(&I) -> K,
		mut create: impl FnMut(&I) -> Result<T, Error>,
		mut update: impl FnMut(&mut T, &I),
	) -> Result<(), Error> {
		let keys: Vec<K> = items.iter().map(&key).collect();
		let unique: HashSet<&K> = keys.iter().collect();
		if unique.len() != keys.len() {
			return Err(Error::new("duplicate key in the items to synchronize"));
		}

		// drops the components of the items that are gone, outside any borrow of `entries`
		let gone: Vec<K> = self.entries.keys().filter(|key| !unique.contains(key)).cloned().collect();
		let removed: Vec<T> = gone.iter().filter_map(|key| self.entries.remove(key)).collect();
		drop(removed);
		// what is still in the DOM, in order: new components are not in it until they are placed
		self.order.retain(|key| unique.contains(key));
		let before: HashMap<&K, usize> =
			self.order.iter().enumerate().map(|(position, key)| (key, position)).collect();
		let before: Vec<Option<usize>> = keys.iter().map(|key| before.get(key).copied()).collect();

		for (item, key) in items.iter().zip(&keys) {
			match self.entries.get_mut(key) {
				Some(component) => update(component, item),
				None => {
					let component = create(item)?;
					self.entries.insert(key.clone(), component);
				}
			}
		}

		let marker = self.marker_in(parent)?;
		let stays = longest_increasing(&before);
		// from the end: an element that stays is the anchor for those before it, any other is put
		// right before the anchor after it
		let mut next = marker;
		for (position, key) in keys.iter().enumerate().rev() {
			let element = self.entries[key].element();
			if !stays[position] {
				parent.insert_before(element, Some(&next))?;
			}
			next = element.clone().into();
		}
		self.order = keys;
		Ok(())
	}

	/// The component of the item with this key, if there is one.
	pub fn get(&self, key: &K) -> Option<&T> {
		self.entries.get(key)
	}

	/// The number of components.
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Whether there are no components.
	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	/// Drop every component.
	pub fn clear(&mut self) {
		let all = std::mem::take(&mut self.entries);
		self.order.clear();
		drop(all);
	}

	/// The end marker, created on first use and re-created if the parent changed.
	fn marker_in(&mut self, parent: &HtmlElement) -> Result<Node, Error> {
		if let Some(marker) = &self.marker {
			if marker.parent_node().is_some_and(|node| node.is_same_node(Some(parent))) {
				return Ok(marker.clone());
			}
		}
		let document = parent.owner_document().ok_or_else(|| Error::new("element has no document"))?;
		let marker: Node = document.create_comment("").into();
		parent.insert_before(&marker, self.before.as_ref())?;
		self.marker = Some(marker.clone());
		Ok(marker)
	}
}

/// Marks the positions of one longest strictly increasing run among the `Some` values: the
/// elements that are already in the right relative order and need not move.
fn longest_increasing(values: &[Option<usize>]) -> Vec<bool> {
	// tails[k]: position of the smallest value ending an increasing run of length k + 1
	let mut tails: Vec<usize> = Vec::new();
	let mut previous: Vec<Option<usize>> = vec![None; values.len()];
	for (position, value) in values.iter().enumerate() {
		let Some(value) = value else { continue };
		let length = tails.partition_point(|tail| values[*tail].is_some_and(|other| other < *value));
		previous[position] = length.checked_sub(1).map(|index| tails[index]);
		if length == tails.len() {
			tails.push(position);
		} else {
			tails[length] = position;
		}
	}
	let mut stays = vec![false; values.len()];
	let mut at = tails.last().copied();
	while let Some(position) = at {
		stays[position] = true;
		at = previous[position];
	}
	stays
}
