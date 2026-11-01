use std::collections::VecDeque;

// a first in, first out queue with a fixed capacity; a push into a full queue evicts the oldest item
#[derive(Debug, Clone)]
pub struct BoundedQueue<T> {
    capacity: usize,
    items: VecDeque<T>,
    dropped: u64,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, items: VecDeque::with_capacity(capacity), dropped: 0 }
    }

    pub fn push(&mut self, item: T) {
        if self.capacity == 0 {
            self.dropped += 1;
            return;
        }
        if self.items.len() >= self.capacity {
            self.items.pop_front();
            self.dropped += 1;
        }
        self.items.push_back(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // items evicted since the queue was made
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
