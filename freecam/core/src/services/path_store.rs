use crate::structs::PathKey;

pub const PATH_CAPACITY: usize = 256;

pub struct PathStore {
    keys: Vec<PathKey>,
    capacity: usize,
    revision: u64,
}

impl PathStore {
    pub fn new(capacity: usize) -> Self {
        Self { keys: Vec::with_capacity(capacity), capacity, revision: 0 }
    }

    // puts the key at `index`, or at the end when the index is past it; false when the store is full
    pub fn insert_at(&mut self, index: usize, key: PathKey) -> bool {
        if self.keys.len() >= self.capacity {
            return false;
        }
        self.keys.insert(index.min(self.keys.len()), key);
        self.revision += 1;
        true
    }

    // returns false when the index is past the end
    pub fn replace_at(&mut self, index: usize, key: PathKey) -> bool {
        let Some(slot) = self.keys.get_mut(index) else {
            return false;
        };
        *slot = key;
        self.revision += 1;
        true
    }

    // returns false when the index is past the end
    pub fn remove_at(&mut self, index: usize) -> bool {
        if index >= self.keys.len() {
            return false;
        }
        self.keys.remove(index);
        self.revision += 1;
        true
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.revision += 1;
    }

    pub fn keys(&self) -> &[PathKey] {
        &self.keys
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    // grows on every successful change
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
