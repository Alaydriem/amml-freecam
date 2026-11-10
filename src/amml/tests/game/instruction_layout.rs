use alaydriem_freecam::game::{InstructionLayout, PacketFactory};
use alaydriem_freecam::types::FreecamEntry;
use amml_game::EntryId;

use crate::support::{FakeGame, FakeResolver};

fn with(id: EntryId, value: u64) -> Vec<(EntryId, u64)> {
    FakeGame::values().into_iter().map(|(known, old)| if known == id { (known, value) } else { (known, old) }).collect()
}

#[test]
fn the_fake_layout_fits() {
    assert!(InstructionLayout::new(&FakeGame::entries()).is_some());
}

#[test]
fn a_front_that_is_not_the_mirror_refuses_the_layout() {
    let values = with(FreecamEntry::INSTRUCTION_SPLINE, 0xB8);

    assert!(InstructionLayout::new(&FakeResolver::entries(&values)).is_none());
}

#[test]
fn a_packet_size_that_is_not_the_mirror_refuses_the_layout() {
    let values = with(FreecamEntry::INSTRUCTION_PACKET_SIZE, 0x190);

    assert!(InstructionLayout::new(&FakeResolver::entries(&values)).is_none());
}

#[test]
fn no_factory_gives_no_packet() {
    assert!(PacketFactory::new(0).create(300, PacketFactory::CAMERA_INSTRUCTION_NAME).is_none());
}

#[test]
fn a_packet_of_another_name_is_given_back() {
    let factory = PacketFactory::new(FakeGame::create_packet_address());

    let packet = factory.create(FakeGame::OTHER_ID, PacketFactory::CAMERA_INSTRUCTION_NAME);

    assert!(packet.is_none());
    assert_eq!((FakeGame::creates(), FakeGame::destroys()), (1, 1));
}

#[test]
fn a_packet_of_the_asked_name_is_kept() {
    let factory = PacketFactory::new(FakeGame::create_packet_address());

    let packet = factory.create(FakeGame::INSTRUCTION_ID, PacketFactory::CAMERA_INSTRUCTION_NAME);

    assert!(packet.is_some());
    assert_eq!(FakeGame::destroys(), 0);
}
