//! Typed index-based arena storage.

use std::marker::PhantomData;

/// Strongly-typed identifier pointing to an element in an `Arena<T>`.
#[derive(Debug)]
pub struct Id<T> {
    index: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> PartialEq for Id<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Id<T> {
    #[inline]
    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn raw(&self) -> u32 {
        self.index
    }
}

/// A contiguous, index-backed arena for storing hierarchical or graph data safely.
#[derive(Debug, Clone)]
pub struct Arena<T> {
    items: Vec<Option<T>>,
    active_count: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            active_count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            active_count: 0,
        }
    }

    #[inline]
    pub fn alloc(&mut self, item: T) -> Id<T> {
        let index = self.items.len() as u32;
        self.items.push(Some(item));
        self.active_count += 1;
        Id::from_raw(index)
    }

    #[inline]
    pub fn get(&self, id: Id<T>) -> Option<&T> {
        self.items.get(id.index as usize)?.as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        self.items.get_mut(id.index as usize)?.as_mut()
    }

    #[inline]
    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        let opt = self.items.get_mut(id.index as usize)?.take();
        if opt.is_some() {
            self.active_count -= 1;
        }
        opt
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.active_count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.active_count == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(idx, opt)| opt.as_ref().map(|item| (Id::from_raw(idx as u32), item)))
    }
}
