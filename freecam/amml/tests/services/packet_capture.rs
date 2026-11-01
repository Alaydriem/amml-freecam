use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use alaydriem_freecam::services::PacketCapture;

use crate::support::FakeGame;

fn handler() -> *mut c_void {
    FakeGame::handler()
}

#[test]
fn a_new_handler_raises_the_generation() {
    let capture = PacketCapture::new();
    let id = FakeGame::network_id();

    capture.capture(handler(), id);
    let first = capture.generation();
    let same = {
        let handler = handler();
        capture.capture(handler, id);
        capture.capture(handler, id);
        capture.generation()
    };

    assert_eq!(first, 1);
    assert_eq!(same, 2);
}

// Review Focus 1: the player leaves, the handler goes with it
#[test]
fn a_player_that_goes_away_drops_the_handler_and_the_next_capture_is_a_new_generation() {
    let capture = PacketCapture::new();
    let old = handler();
    capture.capture(old, FakeGame::network_id());
    capture.observe_local_player(true);
    let generation = capture.generation();

    capture.observe_local_player(false);
    let calls = AtomicUsize::new(0);
    let delivered = capture.deliver(|_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        true
    });

    assert!(!delivered);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(!capture.captured());
    assert_eq!(capture.generation(), 0);

    // the same address again is still a new handler: the old one was dropped
    capture.capture(old, FakeGame::network_id());
    assert_eq!(capture.generation(), generation + 1);
}

#[test]
fn a_delivery_on_the_capture_thread_reaches_the_handler_with_its_id() {
    let capture = PacketCapture::new();
    let (target, id) = (handler(), FakeGame::network_id());
    capture.capture(target, id);

    let mut seen = None;
    assert!(capture.deliver(|handler, identifier| {
        seen = Some((handler, identifier));
        true
    }));

    assert_eq!(seen, Some((target, id)));
}

// Review Focus 4: a delivery on another thread is refused, calls nothing and logs one line
#[test]
fn a_delivery_on_another_thread_is_refused_once_logged_and_calls_nothing() {
    let capture = Arc::new(PacketCapture::new());
    capture.capture(handler(), FakeGame::network_id());
    let calls = Arc::new(AtomicUsize::new(0));

    let other = {
        let (capture, calls) = (Arc::clone(&capture), Arc::clone(&calls));
        std::thread::spawn(move || {
            let mut delivered = Vec::new();
            for _ in 0..3 {
                delivered.push(capture.deliver(|_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    true
                }));
            }
            (delivered, capture.delivery_generation())
        })
    };
    let (delivered, generation) = other.join().unwrap();

    assert_eq!(delivered, [false, false, false]);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(generation, 0);
    assert_eq!(capture.refusals(), 1);
    assert_eq!(capture.delivery_generation(), 1);
}

#[test]
fn a_capture_inside_a_delivery_takes_the_lock_again() {
    let capture = PacketCapture::new();
    let target = handler();
    capture.capture(target, FakeGame::network_id());

    let delivered = capture.deliver(|handler, id| {
        capture.capture(handler, id);
        true
    });

    assert!(delivered);
    assert_eq!(capture.generation(), 1);
}

#[test]
fn reset_drops_the_handler() {
    let capture = PacketCapture::new();
    capture.capture(handler(), FakeGame::network_id());

    capture.reset();

    assert!(!capture.captured());
}
