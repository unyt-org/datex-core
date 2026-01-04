use crate::collections::HashMap;
use crate::collections::hash_map::{Iter, IterMut};
use crate::stdlib::vec::Vec;
use core::prelude::rust_2024::*;

pub trait NextKey: Copy + Eq + core::hash::Hash + Default {
    fn next_key(&mut self) -> Self;
}

/// A HashMap that reuses freed IDs for new entries.
pub struct FreeHashMap<K: NextKey, T> {
    entries: HashMap<K, T>,
    free_list: Vec<K>,
    next_id: K,
}

impl<K: NextKey, T> Default for FreeHashMap<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: NextKey, T> FreeHashMap<K, T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            free_list: Vec::new(),
            next_id: K::default(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.free_list.clear();
        self.next_id = K::default();
    }

    /// Adds a new entry and returns its unique ID.
    pub fn add(&mut self, value: T) -> K {
        if let Some(id) = self.free_list.pop() {
            self.entries.insert(id, value);
            id
        } else {
            let id = self.next_id.next_key();
            self.entries.insert(id, value);
            id
        }
    }

    /// Checks if a value exists in the map.
    pub fn has_value(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.entries.values().any(|v| v == value)
    }

    /// Returns the ID of a given value, if it exists.
    pub fn get_id(&self, value: &T) -> Option<K>
    where
        T: PartialEq,
    {
        for (k, v) in &self.entries {
            if v == value {
                return Some(*k);
            }
        }
        None
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Checks if an entry with the given ID exists.
    pub fn has(&self, id: &K) -> bool {
        self.entries.contains_key(id)
    }

    /// Removes the entry with the given ID, if it exists.
    pub fn remove(&mut self, id: K) -> Option<T> {
        let cur = self.entries.remove(&id);
        if cur.is_some() {
            self.free_list.push(id);
        }
        cur
    }

    /// Get a reference to an entry.
    pub fn get(&self, id: &K) -> Option<&T> {
        self.entries.get(id)
    }

    /// Get a mutable reference to an entry.
    pub fn get_mut(&mut self, id: &K) -> Option<&mut T> {
        self.entries.get_mut(id)
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.values_mut()
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }

    pub fn iter(&self) -> Iter<'_, K, T> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, T> {
        self.entries.iter_mut()
    }
}

impl<K: NextKey, T> IntoIterator for FreeHashMap<K, T> {
    type Item = (K, T);
    type IntoIter = crate::collections::hash_map::IntoIter<K, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, K: NextKey, T> IntoIterator for &'a FreeHashMap<K, T> {
    type Item = (&'a K, &'a T);
    type IntoIter = Iter<'a, K, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<'a, K: NextKey, T> IntoIterator for &'a mut FreeHashMap<K, T> {
    type Item = (&'a K, &'a mut T);
    type IntoIter = IterMut<'a, K, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter_mut()
    }
}

impl NextKey for u32 {
    fn next_key(&mut self) -> Self {
        let current = *self;
        *self = self.wrapping_add(1);
        current
    }
}
