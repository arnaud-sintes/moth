/// Stable handle to an entry of a [`Children`] container.
///
/// The generation makes a stale key (its entry removed, its slot possibly reused) detectable:
/// lookups with it return `None` rather than another child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
	index: u32,
	generation: u32,
}

struct Slot<T> {
	generation: u32,
	value: Option<T>,
}

/// Owner of dynamically created children (rows of a list, items of a menu...).
///
/// Removing an entry drops it, which tears down its DOM element and listeners.
pub struct Children<T> {
	slots: Vec<Slot<T>>,
	free: Vec<u32>,
}

impl<T> Default for Children<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Children<T> {
	/// An empty container.
	pub fn new() -> Self {
		Self { slots: Vec::new(), free: Vec::new() }
	}

	/// Take ownership of `value` and return the [`Key`] that finds it again. A slot freed by
	/// [`Children::remove`] or [`Children::clear`] is reused, with a new generation, so the keys of
	/// removed entries stay invalid. Panics if there would be more than `u32::MAX` slots.
	pub fn insert(&mut self, value: T) -> Key {
		match self.free.pop() {
			Some(index) => {
				let slot = &mut self.slots[index as usize];
				slot.value = Some(value);
				Key { index, generation: slot.generation }
			}
			None => {
				let index = u32::try_from(self.slots.len()).expect("more than u32::MAX children");
				self.slots.push(Slot { generation: 0, value: Some(value) });
				Key { index, generation: 0 }
			}
		}
	}

	/// Take an entry out. Dropping the returned value destroys the child.
	pub fn remove(&mut self, key: Key) -> Option<T> {
		let slot = self.slots.get_mut(key.index as usize)?;
		if slot.generation != key.generation {
			return None;
		}
		let value = slot.value.take()?;
		slot.generation = slot.generation.wrapping_add(1);
		self.free.push(key.index);
		Some(value)
	}

	/// The entry `key` was returned for, or `None` if it has been removed.
	pub fn get(&self, key: Key) -> Option<&T> {
		let slot = self.slots.get(key.index as usize)?;
		if slot.generation != key.generation {
			return None;
		}
		slot.value.as_ref()
	}

	/// Whether `key` still refers to an entry.
	pub fn contains(&self, key: Key) -> bool {
		self.get(key).is_some()
	}

	/// The number of entries.
	pub fn len(&self) -> usize {
		self.slots.len() - self.free.len()
	}

	/// Whether there are no entries.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Every entry with its key, in slot order (not necessarily insertion order, since freed slots
	/// are reused).
	pub fn iter(&self) -> impl Iterator<Item = (Key, &T)> {
		self.slots.iter().enumerate().filter_map(|(index, slot)| {
			let value = slot.value.as_ref()?;
			Some((Key { index: index as u32, generation: slot.generation }, value))
		})
	}

	/// Drop every child, in insertion-slot order.
	pub fn clear(&mut self) {
		for (index, slot) in self.slots.iter_mut().enumerate() {
			if slot.value.take().is_some() {
				slot.generation = slot.generation.wrapping_add(1);
				self.free.push(index as u32);
			}
		}
	}
}
