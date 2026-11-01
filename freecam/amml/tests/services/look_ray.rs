use std::ffi::c_void;
use std::sync::Arc;

use alaydriem_freecam::game::LookLayout;
use alaydriem_freecam::services::{LookRay, PacketCapture};
use freecam_core::structs::Vec3;

use crate::support::FakeGame;

fn look() -> LookRay {
    LookRay::new(LookLayout::new(&FakeGame::entries()).unwrap())
}

fn captured() -> PacketCapture {
    let capture = PacketCapture::new();
    capture.capture(FakeGame::handler(), FakeGame::network_id());
    capture
}

// a ray from the origin along +x for 20 blocks
#[allow(unsafe_code)]
fn cast(look: &LookRay, capture: &PacketCapture, player: *mut c_void, region: *mut c_void) -> Option<Vec3> {
    // SAFETY: fake player and region
    unsafe { look.cast(capture, player, region, Vec3::new(0.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0), 20.0) }
}

#[test]
fn a_block_hit_with_no_actor_in_front_is_the_answer() {
    let region = FakeGame::region(Some([10.0, 0.5, 0.5]), &[]);

    let hit = cast(&look(), &captured(), FakeGame::actor([0.0; 6]), region);

    assert_eq!(hit, Some(Vec3::new(10.0, 0.5, 0.5)));
    assert_eq!((FakeGame::clips(), FakeGame::fetches()), (1, 1));
}

#[test]
fn the_nearest_actor_in_front_of_the_block_wins_and_the_player_is_never_a_target() {
    let player = FakeGame::actor([-1.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    let near = FakeGame::actor([4.0, 0.0, 0.0, 5.0, 1.0, 1.0]);
    let far = FakeGame::actor([7.0, 0.0, 0.0, 8.0, 1.0, 1.0]);
    let region = FakeGame::region(Some([10.0, 0.5, 0.5]), &[player, far, near]);

    let hit = cast(&look(), &captured(), player, region).unwrap();

    assert!((hit.x - (4.0 - 0.1)).abs() < 1e-5, "{hit:?}");
}

#[test]
fn a_miss_is_none() {
    let region = FakeGame::region(None, &[]);

    assert_eq!(cast(&look(), &captured(), FakeGame::actor([0.0; 6]), region), None);
}

#[test]
fn a_ray_that_is_not_finite_calls_nothing() {
    let look = look();
    let region = FakeGame::region(Some([1.0, 1.0, 1.0]), &[]);
    // SAFETY: fake player and region
    #[allow(unsafe_code)]
    let hit = unsafe {
        look.cast(
            &captured(),
            std::ptr::null_mut(),
            region,
            Vec3::new(f32::NAN, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            5.0,
        )
    };

    assert_eq!(hit, None);
    assert_eq!(FakeGame::clips(), 0);
}

// Review Focus 4: a ray on another thread is a miss, calls no game function and logs one line
#[test]
fn a_ray_on_another_thread_is_a_miss_with_no_game_call_and_one_log_line() {
    let look = Arc::new(look());
    let capture = Arc::new(captured());
    let region = FakeGame::region(Some([10.0, 0.5, 0.5]), &[]) as usize;

    let hits = {
        let (look, capture) = (Arc::clone(&look), Arc::clone(&capture));
        std::thread::spawn(move || {
            (0..3).map(|_| cast(&look, &capture, std::ptr::null_mut(), region as *mut c_void)).collect::<Vec<_>>()
        })
        .join()
        .unwrap()
    };

    assert_eq!(hits, [None, None, None]);
    assert_eq!((FakeGame::clips(), FakeGame::fetches()), (0, 0));
    assert_eq!(look.refusals(), 1);
}
