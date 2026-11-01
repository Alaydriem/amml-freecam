use std::ffi::c_void;

use alaydriem_freecam::services::{
    FrameDriver, FrameStatusReader, FreecamGame, FreecamSession, PacketCapture, RenderOverride,
};
use alaydriem_freecam::types::FrameContext;
use amml_game::{ClientInstance, StopLatch};

use crate::support::{FakeGame, FakeHeap, SessionFixture};

struct Shutdown {
    game: FreecamGame,
    fixture: SessionFixture,
    capture: PacketCapture,
    render: RenderOverride,
    reader: FrameStatusReader,
    live: usize,
}

impl Shutdown {
    // freecam on with a path whose markers are live, a handler captured on this thread and the free preset known
    #[allow(unsafe_code)]
    fn new() -> Self {
        let game = FreecamGame::new(FakeGame::entries(), Some(FakeHeap::msvc())).unwrap();
        let fixture = SessionFixture::with_config(SessionFixture::NO_LOOK_AT);
        let (_, world) = fixture.build_path(1);
        let capture = PacketCapture::new();
        capture.capture(FakeGame::handler(), FakeGame::network_id());
        // SAFETY: a fake level with the fake preset list
        assert!(unsafe { game.instructions.ready(&capture, FakeGame::level(&["minecraft:free"])) });
        Self {
            game,
            fixture,
            capture,
            render: RenderOverride::new(),
            reader: FrameStatusReader::new(),
            live: world.len(),
        }
    }

    fn context(&self) -> FrameContext<'_> {
        FrameContext {
            game: &self.game,
            session: self.session(),
            capture: &self.capture,
            render: &self.render,
            reader: &self.reader,
            camera_owned: false,
        }
    }

    fn session(&self) -> &FreecamSession {
        self.fixture.session()
    }

    #[allow(unsafe_code)]
    fn run(&self, player: *mut c_void, latch: &StopLatch) -> (bool, usize) {
        let client_object = FakeGame::client(player);
        // SAFETY: a fake client whose local player slot returns `player`
        let client = unsafe { ClientInstance::from_raw(client_object, &self.game.entries) }.unwrap();
        let mut driver = FrameDriver::new(self.session().marker_capacity());
        driver.shutdown(self.context(), Some(&client), true, latch)
    }
}

// Review Focus 3: disable with the camera on and markers shown
#[test]
fn shutdown_sends_one_clear_and_one_remove_per_live_marker() {
    let shutdown = Shutdown::new();
    assert!(shutdown.live > 0);

    let sent = shutdown.run(FakeGame::actor([0.0; 6]), &StopLatch::new());

    assert_eq!(sent, (true, shutdown.live));
    let clears: Vec<bool> = FakeGame::instructions().iter().map(|seen| seen.clear).collect();
    assert_eq!(clears, [true]);
    let batches = FakeGame::shape_batches();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), shutdown.live);
    assert!(batches[0].iter().all(|shape| shape.shape_type.is_none()));
}

#[test]
fn shutdown_sends_nothing_once_the_stop_latch_is_raised() {
    let shutdown = Shutdown::new();
    let latch = StopLatch::new();
    latch.raise();

    let sent = shutdown.run(FakeGame::actor([0.0; 6]), &latch);

    assert_eq!(sent, (false, 0));
    assert!(FakeGame::instructions().is_empty());
    assert!(FakeGame::shape_batches().is_empty());
    assert_eq!(FakeGame::creates(), 0);
}

#[test]
fn shutdown_sends_nothing_without_a_local_player() {
    let shutdown = Shutdown::new();

    let sent = shutdown.run(std::ptr::null_mut(), &StopLatch::new());

    assert_eq!(sent, (false, 0));
    assert!(FakeGame::instructions().is_empty());
    assert!(FakeGame::shape_batches().is_empty());
}
