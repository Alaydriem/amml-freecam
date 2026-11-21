use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::types::InputItem;

// the inputs of the key and mouse listeners until the next frame drains them. The listeners may run on the input
// thread, so the queue is locked only for one push or one drain. A mouse move right after a mouse move adds into it,
// so a fast mouse fills one item.
//
// Bound: at most `capacity` items. A push into a full queue drops the oldest item that is not a bind, or the new
// item when every queued item is a bind; a bind is never dropped (a lost release would keep a fly key held), and the
// binds are few (one per key event)
pub struct InputQueue {
    items: Mutex<Items>,
    capacity: usize,
}

struct Items {
    queue: VecDeque<InputItem>,
    dropped: u64,
}

impl InputQueue {
    // one frame of mouse buttons, wheel notches and the keys of a chord, with room to spare
    pub const CAPACITY: usize = 256;

    pub fn new(capacity: usize) -> Self {
        Self { items: Mutex::new(Items { queue: VecDeque::with_capacity(capacity), dropped: 0 }), capacity }
    }

    pub fn push(&self, item: InputItem) {
        let mut items = self.lock();
        if let (InputItem::Look { dx, dy }, Some(InputItem::Look { dx: last_dx, dy: last_dy })) =
            (item, items.queue.back_mut())
        {
            *last_dx += dx;
            *last_dy += dy;
            return;
        }
        if items.queue.len() >= self.capacity {
            let oldest = items.queue.iter().position(|queued| !matches!(queued, InputItem::Bind(..)));
            match oldest {
                Some(index) => {
                    items.queue.remove(index);
                    items.dropped += 1;
                }
                None if !matches!(item, InputItem::Bind(..)) => {
                    items.dropped += 1;
                    return;
                }
                None => {}
            }
        }
        items.queue.push_back(item);
    }

    // moves every queued item into `out` (cleared first), oldest first
    pub fn drain(&self, out: &mut Vec<InputItem>) {
        out.clear();
        out.extend(self.lock().queue.drain(..));
    }

    // items dropped because the queue was full
    pub fn dropped(&self) -> u64 {
        self.lock().dropped
    }

    fn lock(&self) -> MutexGuard<'_, Items> {
        self.items.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
