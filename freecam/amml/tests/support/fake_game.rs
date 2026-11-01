use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use alaydriem_freecam::FreecamMod;
use alaydriem_freecam::types::{DeliveringScope, FreecamEntry};
use amml_game::{Actor, CameraWriter, EntryId, GameEntries, Projection};
use amml_msvc::structs::{RefCountBase, RefCountVtable};

use crate::support::{FakeResolver, InstructionSeen, ShapeSeen};

// fake game objects for the freecam services, with every entry the mod asks for. Every call into the fake game is
// counted in this process (nextest runs each test in its own process)
pub struct FakeGame;

static CREATES: AtomicUsize = AtomicUsize::new(0);
static DESTROYS: AtomicUsize = AtomicUsize::new(0);
static CLIPS: AtomicUsize = AtomicUsize::new(0);
static FETCHES: AtomicUsize = AtomicUsize::new(0);
static INSTRUCTIONS: Mutex<Vec<InstructionSeen>> = Mutex::new(Vec::new());
static SHAPES: Mutex<Vec<Vec<ShapeSeen>>> = Mutex::new(Vec::new());
static ON_INSTRUCTION: OnceLock<Box<dyn Fn(usize) + Send + Sync>> = OnceLock::new();
static BLOCK_HIT: Mutex<Option<[f32; 3]>> = Mutex::new(None);
static ACTORS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

static REF_COUNT_VTABLE: RefCountVtable = RefCountVtable {
    destroy: FakeGame::destroy,
    delete_this: FakeGame::delete_this,
    destructor: FakeGame::destructor,
    get_deleter: FakeGame::get_deleter,
};

impl FakeGame {
    pub const INSTRUCTION_ID: u32 = 300;
    pub const SHAPES_ID: u32 = 328;
    pub const PACKET_BYTES: usize = 0x200;
    // the byte every fresh instruction packet holds, so a write outside the set and clear members shows
    pub const FILL: u8 = 0xAB;
    pub const VTABLE_SLOTS: usize = 256;

    pub const INSTRUCTION: usize = 0x30;
    pub const INSTRUCTION_PACKET_SIZE: usize = 0x188;
    pub const SET: usize = 0x50;
    pub const SET_BYTES: usize = 0x60;
    pub const SET_ENGAGED: usize = 0xAC;
    pub const PRESET_INDEX: usize = 0x50;
    pub const POS: usize = 0x60;
    pub const ROT: usize = 0x70;
    pub const SPLINE: usize = 0xB0;
    pub const CLEAR: usize = 0x148;
    // an id the fake factory answers with a packet of another name
    pub const OTHER_ID: u32 = 999;

    pub const SHAPES_VECTOR: usize = 0x30;
    pub const PAYLOAD_SIZE: usize = 0xB8;
    pub const PAYLOAD_TYPE: usize = 0x08;
    pub const PAYLOAD_EXTRA_INDEX: usize = 0xB0;

    pub const HANDLE_SHAPES: usize = 33;
    pub const HANDLE_CHUNK: usize = 135;
    pub const HANDLE_PRESETS: usize = 145;
    pub const HANDLE_INSTRUCTION: usize = 147;
    pub const GET_PRESETS: usize = 29;
    pub const PRESET_SIZE: usize = 0x130;
    pub const GET_LOCAL_PLAYER: usize = 5;
    pub const GET_NAME: usize = 2;
    pub const CLIP: usize = 51;
    pub const FETCH: usize = 31;
    pub const HIT_TYPE: usize = 0x18;
    pub const HIT_POS: usize = 0x2C;
    pub const ACTOR_SHAPE: usize = 0x220;

    // every entry the mod asks for, each with a value the fakes understand
    pub fn values() -> Vec<(EntryId, u64)> {
        let mut values: Vec<(EntryId, u64)> = vec![
            (FreecamEntry::CLIENT_NETWORK_HANDLER_VTABLE, Self::handler_vtable() as u64),
            (FreecamEntry::HANDLE_CHUNK_PUBLISHER, Self::HANDLE_CHUNK as u64),
            (FreecamEntry::HANDLE_CAMERA_PRESETS, Self::HANDLE_PRESETS as u64),
            (FreecamEntry::HANDLE_CAMERA_INSTRUCTION, Self::HANDLE_INSTRUCTION as u64),
            (FreecamEntry::HANDLE_PRIMITIVE_SHAPES, Self::HANDLE_SHAPES as u64),
            (FreecamEntry::CREATE_PACKET, Self::create_packet as *const () as usize as u64),
            (FreecamEntry::INSTRUCTION_PACKET_INSTRUCTION, Self::INSTRUCTION as u64),
            (FreecamEntry::INSTRUCTION_PACKET_SIZE, Self::INSTRUCTION_PACKET_SIZE as u64),
            (FreecamEntry::INSTRUCTION_SPLINE, Self::SPLINE as u64),
            (FreecamEntry::LEVEL_GET_CAMERA_PRESETS, Self::GET_PRESETS as u64),
            (FreecamEntry::CAMERA_PRESET_SIZE, Self::PRESET_SIZE as u64),
            (FreecamEntry::SHAPES_PACKET_SHAPES, Self::SHAPES_VECTOR as u64),
            (FreecamEntry::PAYLOAD_SIZE, Self::PAYLOAD_SIZE as u64),
            (FreecamEntry::GET_CAMERA_REGISTRY, 190),
            (FreecamEntry::CAMERA_ENTITIES, 0x1D8),
            (FreecamEntry::CAMERA_ENTITY_STRIDE, 0x20),
            (FreecamEntry::CAMERA_COMPONENT_TYPE_ID, 0x4F60_47C7),
            (FreecamEntry::CAMERA_COMPONENT_SIZE_FACTOR, 9),
            (FreecamEntry::CAMERA_COMPONENT_SIZE_SHIFT, 5),
            (FreecamEntry::ACTIVE_CAMERA_TYPE_ID, 0x1130_40A1),
            (FreecamEntry::BLOCK_SOURCE_CLIP, Self::CLIP as u64),
            (FreecamEntry::BLOCK_SOURCE_FETCH_ENTITIES, Self::FETCH as u64),
            (FreecamEntry::ACTOR_AABB_SHAPE, Self::ACTOR_SHAPE as u64),
            (EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER, Self::GET_LOCAL_PLAYER as u64),
            (EntryId::CAMERA_FORWARD, 0x118),
            (EntryId::CAMERA_UPDATE_VIEW_MATRIX_DEPENDENCIES, Self::update_camera as *const () as usize as u64),
        ];
        let filler = FreecamMod::REQUIRED
            .iter()
            .chain(CameraWriter::ENTRIES.iter())
            .chain(Actor::EXTRA_ENTRIES.iter())
            .chain(Projection::ENTRIES.iter());
        for id in filler {
            if !values.iter().any(|(known, _)| known == id) {
                values.push((*id, 0x40));
            }
        }
        values
    }

    pub fn entries() -> GameEntries {
        FreecamMod::resolve(&FakeResolver::new(&Self::values())).unwrap()
    }

    pub fn create_packet_address() -> usize {
        Self::create_packet as *const () as usize
    }

    // a ClientNetworkHandler: its first qword is the fake vtable
    pub fn handler() -> *mut c_void {
        Box::leak(Box::new([Self::handler_vtable(), 0usize])).as_mut_ptr().cast()
    }

    // a NetworkIdentifier; never read by the fakes
    pub fn network_id() -> *const c_void {
        Box::leak(Box::new([0u8; 0xA0])).as_ptr().cast()
    }

    // a Level whose preset list holds `names`
    pub fn level(names: &[&str]) -> *mut c_void {
        let words: &mut [u64] = Box::leak(vec![0u64; names.len().max(1) * Self::PRESET_SIZE / 8].into_boxed_slice());
        let first = words.as_mut_ptr() as usize;
        // SAFETY: the words are a live buffer of this many bytes
        let presets = unsafe { std::slice::from_raw_parts_mut(first as *mut u8, words.len() * 8) };
        for (index, name) in names.iter().enumerate() {
            assert!(name.len() <= 15);
            let preset = &mut presets[index * Self::PRESET_SIZE..];
            preset[..name.len()].copy_from_slice(name.as_bytes());
            preset[16..24].copy_from_slice(&name.len().to_le_bytes());
            preset[24..32].copy_from_slice(&15usize.to_le_bytes());
        }
        let last = first + names.len() * Self::PRESET_SIZE;
        let list: &mut [usize; 3] = Box::leak(Box::new([first, last, last]));
        let vtable = Self::vtable(&[(Self::GET_PRESETS, Self::get_presets as *const () as usize)]);
        Box::leak(Box::new([vtable, list.as_ptr() as usize])).as_mut_ptr().cast()
    }

    // a ClientInstance whose local player is `player`
    pub fn client(player: *mut c_void) -> *mut c_void {
        let vtable = Self::vtable(&[(Self::GET_LOCAL_PLAYER, Self::get_local_player as *const () as usize)]);
        Box::leak(Box::new([vtable, player as usize])).as_mut_ptr().cast()
    }

    // an Actor of zeros; only its address and its box are read
    pub fn actor(bounds: [f32; 6]) -> *mut c_void {
        let shape: &mut [f32; 6] = Box::leak(Box::new(bounds));
        let actor: &mut [u8] = Box::leak(vec![0u8; 0x400].into_boxed_slice());
        actor[Self::ACTOR_SHAPE..Self::ACTOR_SHAPE + 8].copy_from_slice(&(shape.as_ptr() as usize).to_le_bytes());
        actor.as_mut_ptr().cast()
    }

    // a BlockSource whose clip hits `block` (None: no hit) and whose actor list is `actors`
    pub fn region(block: Option<[f32; 3]>, actors: &[*mut c_void]) -> *mut c_void {
        *BLOCK_HIT.lock().unwrap() = block;
        *ACTORS.lock().unwrap() = actors.iter().map(|actor| *actor as usize).collect();
        let vtable = Self::vtable(&[
            (Self::CLIP, Self::clip as *const () as usize),
            (Self::FETCH, Self::fetch as *const () as usize),
        ]);
        Box::leak(Box::new([vtable])).as_mut_ptr().cast()
    }

    // what the instruction handler saw for each packet it got
    pub fn instructions() -> Vec<InstructionSeen> {
        INSTRUCTIONS.lock().unwrap().clone()
    }

    // the shapes of each PrimitiveShapesPacket the handler got
    pub fn shape_batches() -> Vec<Vec<ShapeSeen>> {
        SHAPES.lock().unwrap().clone()
    }

    pub fn creates() -> usize {
        CREATES.load(Ordering::SeqCst)
    }

    pub fn destroys() -> usize {
        DESTROYS.load(Ordering::SeqCst)
    }

    pub fn clips() -> usize {
        CLIPS.load(Ordering::SeqCst)
    }

    pub fn fetches() -> usize {
        FETCHES.load(Ordering::SeqCst)
    }

    // runs `body` with the packet address each time the instruction handler is called
    pub fn on_instruction(body: impl Fn(usize) + Send + Sync + 'static) {
        assert!(ON_INSTRUCTION.set(Box::new(body)).is_ok());
    }

    fn handler_vtable() -> usize {
        static VTABLE: OnceLock<usize> = OnceLock::new();
        *VTABLE.get_or_init(|| {
            Self::vtable(&[
                (Self::HANDLE_SHAPES, Self::handle_shapes as *const () as usize),
                (Self::HANDLE_INSTRUCTION, Self::handle_instruction as *const () as usize),
            ])
        })
    }

    fn vtable(slots: &[(usize, usize)]) -> usize {
        let table: &mut [usize] = Box::leak(vec![0usize; Self::VTABLE_SLOTS].into_boxed_slice());
        for (slot, function) in slots {
            table[*slot] = *function;
        }
        table.as_ptr() as usize
    }

    // a packet of PACKET_BYTES bytes, 8-byte aligned, whose first qword is a vtable that answers getName
    extern "C" fn create_packet(out: *mut [usize; 2], id: u32) -> *mut [usize; 2] {
        CREATES.fetch_add(1, Ordering::SeqCst);
        let fill = if id == Self::INSTRUCTION_ID { Self::FILL } else { 0 };
        let words: &mut [u64] =
            Box::leak(vec![u64::from_ne_bytes([fill; 8]); Self::PACKET_BYTES / 8].into_boxed_slice());
        words[0] = Self::packet_vtable(id) as u64;
        // SAFETY: the words are a live buffer of PACKET_BYTES bytes
        let packet = unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), Self::PACKET_BYTES) };
        let control = Box::leak(Box::new(RefCountBase { vtable: &REF_COUNT_VTABLE, uses: 1.into(), weaks: 1.into() }));
        // SAFETY: `out` is the caller's buffer for a shared_ptr: element pointer, then control block
        unsafe { out.write([packet.as_mut_ptr() as usize, std::ptr::from_mut(control) as usize]) };
        out
    }

    fn packet_vtable(id: u32) -> usize {
        static VTABLES: OnceLock<[usize; 3]> = OnceLock::new();
        let [instruction, shapes, other] = *VTABLES.get_or_init(|| {
            [
                Self::vtable(&[(Self::GET_NAME, Self::instruction_name as *const () as usize)]),
                Self::vtable(&[(Self::GET_NAME, Self::shapes_name as *const () as usize)]),
                Self::vtable(&[(Self::GET_NAME, Self::other_name as *const () as usize)]),
            ]
        });
        match id {
            Self::INSTRUCTION_ID => instruction,
            Self::SHAPES_ID => shapes,
            _ => other,
        }
    }

    fn name(out: *mut [usize; 2], name: &'static str) -> *mut [usize; 2] {
        // SAFETY: the mod passes a 16-byte buffer for the string_view
        unsafe { out.write([name.as_ptr() as usize, name.len()]) };
        out
    }

    extern "C" fn instruction_name(_this: *const c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        Self::name(out, "CameraInstructionPacket")
    }

    extern "C" fn shapes_name(_this: *const c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        Self::name(out, "PrimitiveShapesPacket")
    }

    extern "C" fn other_name(_this: *const c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        Self::name(out, "TextPacket")
    }

    extern "C" fn handle_instruction(_this: *mut c_void, _id: *const c_void, packet: *const u8) {
        let base = Self::INSTRUCTION;
        // SAFETY: the mod passes the live fake packet of PACKET_BYTES bytes
        let bytes = unsafe { std::slice::from_raw_parts(packet, Self::PACKET_BYTES) }.to_vec();
        INSTRUCTIONS.lock().unwrap().push(InstructionSeen {
            delivering: DeliveringScope::active(),
            set: bytes[base + Self::SET_ENGAGED] == 1,
            clear: bytes[base + Self::CLEAR] == 1 && bytes[base + Self::CLEAR + 1] == 1,
            bytes,
        });
        if let Some(body) = ON_INSTRUCTION.get() {
            body(packet as usize);
        }
    }

    extern "C" fn handle_shapes(_this: *mut c_void, _id: *const c_void, packet: *const u8) {
        // SAFETY: the mod passes the live fake packet, whose shape vector it filled
        let [first, last, _] = unsafe { packet.add(Self::SHAPES_VECTOR).cast::<[usize; 3]>().read_unaligned() };
        let shapes = (0..(last - first) / Self::PAYLOAD_SIZE)
            .map(|index| {
                let payload = (first + index * Self::PAYLOAD_SIZE) as *const u8;
                // SAFETY: each payload of the vector is PAYLOAD_SIZE bytes
                unsafe {
                    ShapeSeen {
                        id: payload.cast::<u64>().read_unaligned(),
                        shape_type: (payload.add(Self::PAYLOAD_TYPE + 1).read() == 1)
                            .then(|| payload.add(Self::PAYLOAD_TYPE).read()),
                        extra_index: payload.add(Self::PAYLOAD_EXTRA_INDEX).read(),
                    }
                }
            })
            .collect();
        SHAPES.lock().unwrap().push(shapes);
    }

    extern "C" fn get_presets(this: *const usize) -> *const usize {
        // SAFETY: the fake level holds its preset list pointer after its vtable
        unsafe { this.add(1).read() as *const usize }
    }

    extern "C" fn get_local_player(this: *const usize) -> *mut c_void {
        // SAFETY: the fake client holds its player pointer after its vtable
        unsafe { this.add(1).read() as *mut c_void }
    }

    extern "C" fn update_camera(_camera: *mut c_void) {}

    #[allow(clippy::too_many_arguments)]
    extern "C" fn clip(
        _this: *mut c_void,
        out: *mut u8,
        _from: *const [f32; 3],
        _to: *const [f32; 3],
        _liquid: u8,
        _shape: u8,
        _steps: i32,
        _border: u8,
        _full: u8,
        _actor: *mut c_void,
        _check: *const c_void,
        _stop: u8,
    ) -> *mut u8 {
        CLIPS.fetch_add(1, Ordering::SeqCst);
        let hit = *BLOCK_HIT.lock().unwrap();
        // SAFETY: the mod passes a buffer larger than a HitResult
        unsafe {
            out.add(Self::HIT_TYPE).cast::<i32>().write_unaligned(if hit.is_some() { 0 } else { 3 });
            out.add(Self::HIT_POS).cast::<[f32; 3]>().write_unaligned(hit.unwrap_or_default());
        }
        out
    }

    extern "C" fn fetch(
        _this: *mut c_void,
        out: *mut [usize; 2],
        _except: *const c_void,
        _bounds: *const [f32; 6],
        _hitbox: u8,
        _display: u8,
    ) -> *mut [usize; 2] {
        FETCHES.fetch_add(1, Ordering::SeqCst);
        let actors = ACTORS.lock().unwrap();
        let list: &mut [usize] = Box::leak(actors.clone().into_boxed_slice());
        // SAFETY: the mod passes a 16-byte span buffer
        unsafe { out.write([list.len(), list.as_ptr() as usize]) };
        out
    }

    unsafe extern "C" fn destroy(_this: *mut RefCountBase) {
        DESTROYS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn delete_this(_this: *mut RefCountBase) {}

    unsafe extern "C" fn destructor(this: *mut RefCountBase, _flags: u32) -> *mut c_void {
        this.cast()
    }

    unsafe extern "C" fn get_deleter(_this: *const RefCountBase, _info: *const c_void) -> *mut c_void {
        std::ptr::null_mut()
    }
}
