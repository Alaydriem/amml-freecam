use std::sync::{Mutex, MutexGuard, PoisonError};

use amml_msvc::MsvcHeap;
use freecam_core::structs::MarkerEntry;

use crate::game::{PacketHandler, ShapeLayout};
use crate::services::PacketCapture;
use crate::types::ShapeState;

// the path markers, as one PrimitiveShapesPacket the client's own handler draws. The packet is made once; its shape
// vector is a game-heap block that grows to the largest batch and is emptied after each delivery. The payloads are
// trivially destructible, so emptying the vector is moving its end back to its start
pub struct ShapeDelivery {
    layout: ShapeLayout,
    handler: PacketHandler,
    heap: MsvcHeap,
    state: Mutex<ShapeState>,
}

impl ShapeDelivery {
    pub fn new(layout: ShapeLayout, handler: PacketHandler, heap: MsvcHeap) -> Self {
        Self { layout, handler, heap, state: Mutex::new(ShapeState::default()) }
    }

    // draws, moves or removes the shapes of `entries` in order; false when nothing went out
    #[allow(unsafe_code)]
    pub fn deliver(&self, capture: &PacketCapture, entries: &[MarkerEntry]) -> bool {
        if entries.is_empty() || !capture.captured() || !capture.on_capture_thread() {
            return false;
        }
        let mut state = self.lock();
        let ShapeState { packet, first: buffer_first, capacity } = &mut *state;
        let made = packet.get_or_make(|| self.layout.create_packet());
        let Some(packet) = made else {
            curia::warn!("could not create a primitive shapes packet");
            return false;
        };
        let size = ShapeLayout::SIZE;
        let Some(bytes) = entries.len().checked_mul(size) else { return false };
        if *capacity < entries.len() {
            // SAFETY: the heap is the game's; the block backs the packet's vector and is freed by the game's vector
            // when the packet is destroyed
            let first = unsafe { self.heap.allocate(bytes) };
            if first.is_null() {
                return false;
            }
            if !buffer_first.is_null() {
                // SAFETY: the old block came from this heap for `capacity * size` bytes, and the vector is empty
                unsafe { self.heap.deallocate(*buffer_first, *capacity * size) };
            }
            *buffer_first = first;
            *capacity = entries.len();
        }
        let (first, end) = (*buffer_first, (*buffer_first).wrapping_add(*capacity * size));
        let vector = self.layout.vector(packet);
        // SAFETY: the block holds `capacity` payloads; each payload is zeroed (every option disengaged, the extra
        // data the first alternative) and then filled from the entry. The vector then spans the filled payloads
        unsafe {
            std::ptr::write_bytes(first, 0, bytes);
            for (index, entry) in entries.iter().enumerate() {
                self.layout.fill(first.add(index * size), entry);
            }
            vector.write([first, first.add(bytes), end]);
        }
        let delivered = capture.deliver(|handler, id| {
            // SAFETY: the capture holds a live handler and its id under its lock, on the capture thread; the packet
            // and its shapes are live
            unsafe { self.handler.deliver_shapes(handler, id, packet) }
        });
        // SAFETY: the vector of the live packet; its payloads need no destructor, so the end moves back to the start
        unsafe { vector.write([first, first, end]) };
        delivered
    }

    // disable: the packet goes back to the game, which frees the block with its empty vector
    pub fn reset(&self) {
        let mut state = self.lock();
        state.packet.release();
        state.first = std::ptr::null_mut();
        state.capacity = 0;
    }

    fn lock(&self) -> MutexGuard<'_, ShapeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
