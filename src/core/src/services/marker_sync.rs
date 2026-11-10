use crate::structs::{MarkerEntry, MarkerKind, MarkerShape};

// network id of slot 0; slot `k` uses `MARKER_ID_BASE + k`
pub const MARKER_ID_BASE: u64 = 0x4643_0000_0000_0000;

// keeps the client's shapes in step with a target list: diffs slot by slot against what was sent and
// spreads the changes over batches of at most `max_per_send` entries, oldest slot first
//
// Bound: every buffer holds `max_shapes` slots, sized once in `new()`. Shapes past `max_shapes` are
// dropped. A slot has at most one pending change, and a newer target for the slot replaces it. A slot
// that changes kind takes two batches: a remove, then the new shape.
pub struct MarkerSync {
    max_shapes: usize,
    max_per_send: usize,
    // the shape each slot should show, `Remove` for an empty slot
    target: Vec<MarkerShape>,
    // the shape the client was last sent for each slot, `Remove` when nothing is live there
    sent: Vec<MarkerShape>,
    // true when the slot's target is not yet sent
    dirty: Vec<bool>,
    pending: usize,
    live: usize,
    target_len: usize,
    // every slot at or above this index is empty in both `target` and `sent`
    extent: usize,
    dimension: i32,
    // reused for every batch and for the shutdown batch
    batch: Vec<MarkerEntry>,
}

impl MarkerSync {
    pub fn new(max_shapes: usize, max_per_send: usize) -> Self {
        Self {
            max_shapes,
            max_per_send: max_per_send.max(1),
            target: vec![MarkerShape::default(); max_shapes],
            sent: vec![MarkerShape::default(); max_shapes],
            dirty: vec![false; max_shapes],
            pending: 0,
            live: 0,
            target_len: 0,
            extent: 0,
            dimension: 0,
            batch: Vec::with_capacity(max_shapes),
        }
    }

    // replaces the target; pending work is recomputed against what was actually sent
    pub fn set_target(&mut self, shapes: &[MarkerShape], dimension: i32) {
        let count = shapes.len().min(self.max_shapes);
        let dimension_changed = dimension != self.dimension;
        self.dimension = dimension;
        let end = self.extent.max(count);
        for slot in 0..end {
            let shape = shapes.get(slot).copied().unwrap_or_default();
            self.target[slot] = shape;
            let moved_dimension = dimension_changed && !Self::is_empty(&shape);
            self.mark(slot, shape != self.sent[slot] || moved_dimension);
        }
        self.target_len = count;
        self.extent = end;
    }

    // every live slot gets a remove
    pub fn hide(&mut self) {
        self.set_target(&[], self.dimension);
    }

    // the client dropped every shape with the world: nothing is live and the whole target is pending
    pub fn forget_sent(&mut self) {
        for slot in 0..self.extent {
            self.sent[slot] = MarkerShape::default();
            let shape = self.target[slot];
            self.mark(slot, !Self::is_empty(&shape));
        }
        self.live = 0;
        self.extent = self.target_len;
    }

    // the next changes to deliver, at most `max_per_send`, empty when the client is in step;
    // the slice stays valid until the next call that changes the sync
    pub fn next_batch(&mut self) -> &[MarkerEntry] {
        self.batch.clear();
        let mut slot = 0;
        while self.pending > 0 && self.batch.len() < self.max_per_send && slot < self.extent {
            if self.dirty[slot] {
                self.send(slot);
            }
            slot += 1;
        }
        if self.pending == 0 {
            self.extent = self.target_len;
        }
        &self.batch
    }

    // the last batch built by `next_batch` or `shutdown_batch`
    pub fn batch(&self) -> &[MarkerEntry] {
        &self.batch
    }

    // the most entries a batch, the shutdown batch included, can hold
    pub fn capacity(&self) -> usize {
        self.max_shapes
    }

    pub fn live_slots(&self) -> usize {
        self.live
    }

    // a remove for every live slot in one batch, ignoring `max_per_send`; afterwards nothing is
    // live and nothing is pending
    pub fn shutdown_batch(&mut self) -> &[MarkerEntry] {
        self.batch.clear();
        for slot in 0..self.extent {
            if !Self::is_empty(&self.sent[slot]) {
                self.batch.push(MarkerEntry::new(Self::id(slot), MarkerShape::default(), self.dimension));
            }
            self.sent[slot] = MarkerShape::default();
            self.target[slot] = MarkerShape::default();
            self.dirty[slot] = false;
        }
        self.pending = 0;
        self.live = 0;
        self.target_len = 0;
        self.extent = 0;
        &self.batch
    }

    // a live slot that changes kind is removed first and stays pending; the client keeps a shape's
    // first kind on an update, so the new shape goes in a later batch
    fn send(&mut self, slot: usize) {
        let shape = self.target[slot];
        let was_live = !Self::is_empty(&self.sent[slot]);
        let now_live = !Self::is_empty(&shape);
        if was_live && now_live && self.sent[slot].kind != shape.kind {
            self.sent[slot] = MarkerShape::default();
            self.live -= 1;
            self.batch.push(MarkerEntry::new(Self::id(slot), MarkerShape::default(), self.dimension));
            return;
        }
        self.sent[slot] = shape;
        self.mark(slot, false);
        self.live = self.live + usize::from(now_live) - usize::from(was_live);
        self.batch.push(MarkerEntry::new(Self::id(slot), shape, self.dimension));
    }

    fn mark(&mut self, slot: usize, dirty: bool) {
        if self.dirty[slot] != dirty {
            self.dirty[slot] = dirty;
            if dirty {
                self.pending += 1;
            } else {
                self.pending -= 1;
            }
        }
    }

    fn id(slot: usize) -> u64 {
        MARKER_ID_BASE + slot as u64
    }

    fn is_empty(shape: &MarkerShape) -> bool {
        shape.kind == MarkerKind::Remove
    }
}
