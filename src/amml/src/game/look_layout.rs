use std::ffi::c_void;

use amml_game::GameEntries;
use amml_game::game::RawObject;
use amml_msvc::MsvcFunction;

use crate::game::layout::{AabbShapeLayout, HitResultLayout};
use crate::types::FreecamEntry;

// the calls and layouts of the look ray: BlockSource::clip, BlockSource::fetchEntities, the HitResult it returns and
// the box of an actor (the HitResult and AABBShapeComponent mirrors)
#[derive(Debug, Clone, Copy)]
pub struct LookLayout {
    clip: u64,
    fetch: u64,
    actor_shape: u64,
}

// HitResult clip(Vec3 const&, Vec3 const&, bool, ShapeType, int, bool, bool, Actor*, std::function const&, bool)
type ClipFn = unsafe extern "C" fn(
    *mut c_void,
    *mut HitResultLayout,
    *const [f32; 3],
    *const [f32; 3],
    u8,
    u8,
    i32,
    u8,
    u8,
    *mut c_void,
    *const c_void,
    u8,
) -> *mut HitResultLayout;

// gsl::span<gsl::not_null<Actor*>> fetchEntities(Actor const*, AABB const&, bool, bool)
type FetchFn =
    unsafe extern "C" fn(*mut c_void, *mut [usize; 2], *const c_void, *const [f32; 6], u8, u8) -> *mut [usize; 2];

// the result buffer: a HitResult with room to spare, so a build whose HitResult grew does not write past it
#[repr(C)]
struct HitBuffer {
    hit: HitResultLayout,
    _spare: [u64; 0x2F],
}

impl LookLayout {
    // HitResultType::Tile
    pub const HIT_TILE: i32 = 0;
    // ShapeType::Outline: the selection outline of each block
    pub const SHAPE_OUTLINE: u8 = 0;

    pub fn new(entries: &GameEntries) -> Option<Self> {
        Some(Self {
            clip: entries.get(FreecamEntry::BLOCK_SOURCE_CLIP)?,
            fetch: entries.get(FreecamEntry::BLOCK_SOURCE_FETCH_ENTITIES)?,
            actor_shape: entries.get(FreecamEntry::ACTOR_AABB_SHAPE)?,
        })
    }

    /// the block the segment from..to hits first, every block a candidate, at most `steps` voxels; None for a miss.
    /// Game thread, on the thread that handles the server's packets
    ///
    /// # Safety
    /// `region` must be the live BlockSource of the local player's dimension
    #[allow(unsafe_code)]
    pub unsafe fn clip(&self, region: *mut c_void, from: [f32; 3], to: [f32; 3], steps: i32) -> Option<[f32; 3]> {
        // SAFETY: the caller guarantees a live BlockSource whose vtable holds clip at the slot
        let function = unsafe { RawObject::slot(region, self.clip) }?;
        // SAFETY: the slot holds the 10-argument clip
        let clip: ClipFn = unsafe { std::mem::transmute(function) };
        let every_block = MsvcFunction::new(|_: *const c_void, _: *const c_void, _: bool| true);
        // SAFETY: all zero is a valid HitBuffer (numbers, bools and an empty entity reference)
        let mut buffer: HitBuffer = unsafe { std::mem::zeroed() };
        // SAFETY: the buffer is larger than a HitResult; the std::function lives for the call and the game only calls
        // it (it never copies it). A clip without an actor returns only tile hits, whose entity reference is empty,
        // so the result needs no destructor
        unsafe {
            clip(
                region,
                &mut buffer.hit,
                &from,
                &to,
                0,
                Self::SHAPE_OUTLINE,
                steps,
                0,
                0,
                std::ptr::null_mut(),
                every_block.as_ptr(),
                0,
            )
        };
        (buffer.hit.kind == Self::HIT_TILE).then_some(buffer.hit.pos)
    }

    /// the actors the game keeps in `region` whose boxes meet `bounds` (min x, y, z, max x, y, z), as pointers
    ///
    /// # Safety
    /// As `clip`
    #[allow(unsafe_code)]
    pub unsafe fn actors(&self, region: *mut c_void, bounds: [f32; 6]) -> &'static [*mut c_void] {
        // SAFETY: the caller guarantees a live BlockSource whose vtable holds fetchEntities at the slot
        let Some(function) = (unsafe { RawObject::slot(region, self.fetch) }) else { return &[] };
        // SAFETY: the slot holds fetchEntities(Actor const*, AABB const&, bool, bool)
        let fetch: FetchFn = unsafe { std::mem::transmute(function) };
        let mut span = [0usize; 2];
        // SAFETY: the span buffer is 16 bytes; the game fills count and pointer
        unsafe { fetch(region, &mut span, std::ptr::null(), &bounds, 1, 0) };
        let [count, data] = span;
        if data == 0 || count == 0 || count > 1 << 20 {
            return &[];
        }
        // SAFETY: the span points into a member vector of the BlockSource that stays valid until the next fetch
        unsafe { std::slice::from_raw_parts(data as *const *mut c_void, count) }
    }

    /// the box of `actor` (min x, y, z, max x, y, z); None when its shape component is missing
    ///
    /// # Safety
    /// `actor` must be a live Actor
    #[allow(unsafe_code)]
    pub unsafe fn actor_box(&self, actor: *mut c_void) -> Option<[f32; 6]> {
        // SAFETY: the caller guarantees a live Actor that holds the pointer to its AABBShapeComponent at the offset
        let shape = unsafe { RawObject::pointer(actor, self.actor_shape) }?;
        // SAFETY: a live AABBShapeComponent, laid out as the mirror
        Some(unsafe { (*shape.cast::<AabbShapeLayout>()).aabb })
    }
}
