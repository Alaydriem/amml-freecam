use std::sync::{Mutex, OnceLock};

use alaydriem_freecam::game::{InstructionLayout, PacketHandler, PresetList};
use alaydriem_freecam::services::{CameraInstructions, PacketCapture};
use alaydriem_freecam::types::InstructionKind;
use freecam_core::structs::{CameraPose, Vec3};

use crate::support::FakeGame;

fn instructions() -> CameraInstructions {
    let entries = FakeGame::entries();
    CameraInstructions::new(
        InstructionLayout::new(&entries).unwrap(),
        PacketHandler::new(&entries).unwrap(),
        PresetList::new(&entries).unwrap(),
    )
}

fn captured() -> PacketCapture {
    let capture = PacketCapture::new();
    capture.capture(FakeGame::handler(), FakeGame::network_id());
    capture
}

fn pose() -> CameraPose {
    CameraPose::new(Vec3::new(1.5, 70.0, -3.0), 45.0, -10.0)
}

// SAFETY (for the callers): the fake level holds the fake preset list
#[allow(unsafe_code)]
fn ready(instructions: &CameraInstructions, capture: &PacketCapture, names: &[&str]) -> bool {
    // SAFETY: as above
    unsafe { instructions.ready(capture, FakeGame::level(names)) }
}

#[test]
fn ready_needs_the_free_preset_and_a_captured_handler() {
    let instructions = instructions();

    assert!(!ready(&instructions, &PacketCapture::new(), &["minecraft:free"]));
    assert!(ready(&instructions, &captured(), &["minecraft:free"]));
}

// Review Focus 5: no free preset, not ready, no packet
#[test]
fn without_the_free_preset_freecam_is_not_ready_and_no_packet_is_made() {
    let instructions = instructions();
    let capture = captured();

    let is_ready = ready(&instructions, &capture, &["custom:orbit"]);
    let sent = instructions.send_set(&capture, &pose());

    assert!(!is_ready);
    assert!(!sent);
    assert_eq!(FakeGame::creates(), 0);
    assert!(FakeGame::instructions().is_empty());
}

#[test]
fn a_set_writes_only_the_set_member_and_disengages_the_clear() {
    let instructions = instructions();
    let capture = captured();
    ready(&instructions, &capture, &["minecraft:first", "minecraft:free"]);

    assert!(instructions.send_set(&capture, &pose()));

    let seen = FakeGame::instructions();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].set && !seen[0].clear && seen[0].delivering);
    let base = FakeGame::INSTRUCTION;
    let bytes = &seen[0].bytes;
    let f32_at = |at: usize| f32::from_le_bytes(bytes[base + at..base + at + 4].try_into().unwrap());
    assert_eq!(u32::from_le_bytes(bytes[base + FakeGame::PRESET_INDEX..][..4].try_into().unwrap()), 1);
    assert_eq!([f32_at(FakeGame::POS), f32_at(FakeGame::POS + 4), f32_at(FakeGame::POS + 8)], [1.5, 70.0, -3.0]);
    assert_eq!([f32_at(FakeGame::ROT), f32_at(FakeGame::ROT + 4)], [-10.0, 45.0]);
    assert_eq!(bytes[base + FakeGame::CLEAR], 0);
    assert_eq!(bytes[base + FakeGame::SET_ENGAGED], 1);
    // every byte outside the vtable, the set member and the clear option keeps what the factory left
    let set = base + FakeGame::SET..base + FakeGame::SET + FakeGame::SET_BYTES;
    let clear = base + FakeGame::CLEAR..base + FakeGame::CLEAR + 2;
    for (at, byte) in bytes.iter().enumerate().skip(8) {
        if !set.contains(&at) && !clear.contains(&at) {
            assert_eq!(*byte, FakeGame::FILL, "byte {at:#x} changed");
        }
    }
}

#[test]
fn a_clear_engages_the_clear_and_disengages_the_set() {
    let instructions = instructions();
    let capture = captured();
    ready(&instructions, &capture, &["minecraft:free"]);
    instructions.send_set(&capture, &pose());

    assert!(instructions.send_clear(&capture));

    let seen = FakeGame::instructions();
    assert!(seen[1].clear && !seen[1].set);
    assert_eq!(FakeGame::creates(), 1, "one packet serves the session");
}

#[test]
fn reset_gives_the_packet_back_to_the_game() {
    let instructions = instructions();
    let capture = captured();
    ready(&instructions, &capture, &["minecraft:free"]);
    instructions.send_clear(&capture);

    instructions.reset();

    assert_eq!(FakeGame::destroys(), 1);
    assert!(!instructions.send_set(&capture, &pose()), "the preset index is forgotten");
}

// Review Focus 2: the mod's own delivery is not reported; a server instruction is, once
#[test]
#[allow(unsafe_code)]
fn the_mods_own_delivery_is_not_reported_and_a_server_instruction_is_reported_once() {
    static SHARED: OnceLock<CameraInstructions> = OnceLock::new();
    static REPORTS: Mutex<Vec<InstructionKind>> = Mutex::new(Vec::new());
    let instructions = SHARED.get_or_init(instructions);
    FakeGame::on_instruction(|packet| {
        // SAFETY: the fake handler passes the live fake packet
        unsafe {
            SHARED.get().unwrap().report(packet as *const _, |kind| REPORTS.lock().unwrap().push(kind));
        }
    });
    let capture = captured();
    ready(instructions, &capture, &["minecraft:free"]);

    instructions.send_set(&capture, &pose());
    let after_own = REPORTS.lock().unwrap().len();
    let mut packet = vec![0u64; FakeGame::PACKET_BYTES / 8];
    // SAFETY: the words are a live buffer of PACKET_BYTES bytes
    unsafe { packet.as_mut_ptr().cast::<u8>().add(FakeGame::INSTRUCTION + FakeGame::SET_ENGAGED).write(1) };
    // SAFETY: a packet whose set member is engaged, as a server sends it
    unsafe { instructions.report(packet.as_ptr().cast(), |kind| REPORTS.lock().unwrap().push(kind)) };

    assert!(FakeGame::instructions()[0].delivering);
    assert_eq!(after_own, 0);
    assert_eq!(*REPORTS.lock().unwrap(), [InstructionKind::Set]);
}

#[test]
#[allow(unsafe_code)]
fn a_server_clear_is_a_clear_and_a_fade_is_neither() {
    let instructions = instructions();
    let mut packet = vec![0u64; FakeGame::PACKET_BYTES / 8];
    let bytes = packet.as_mut_ptr().cast::<u8>();

    // SAFETY: an empty instruction packet of PACKET_BYTES bytes
    let empty = unsafe { instructions.kind_of(bytes.cast_const().cast()) };
    // SAFETY: as above, with the clear option engaged and true
    let clear = unsafe {
        bytes.add(FakeGame::INSTRUCTION + FakeGame::CLEAR).write(1);
        bytes.add(FakeGame::INSTRUCTION + FakeGame::CLEAR + 1).write(1);
        instructions.kind_of(bytes.cast_const().cast())
    };

    assert_eq!((empty, clear), (InstructionKind::Other, InstructionKind::Clear));
}
