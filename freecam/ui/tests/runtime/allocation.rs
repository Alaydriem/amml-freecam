use stats_alloc::Region;

use crate::GLOBAL;
use crate::support::ui_fixture::UiFixture;

const WARM_UP_FRAMES: usize = 10;
const MEASURED_FRAMES: usize = 200;

// allocations per frame of the views alone, after the warm-up; the mod manager tessellates and copies
fn allocations_per_frame(fixture: &mut UiFixture) -> usize {
    for _ in 0..WARM_UP_FRAMES {
        fixture.run();
    }
    let region = Region::new(&GLOBAL);
    for _ in 0..MEASURED_FRAMES {
        fixture.run();
    }
    region.change().allocations / MEASURED_FRAMES
}

#[test]
fn report_egui_allocations_per_frame() {
    let scenes = [
        ("hidden", UiFixture::new()),
        ("strip + legend", UiFixture::new()),
        ("strip + legend + bar, 3 points", UiFixture::new().with_path()),
        ("strip + legend + bar, 256 points", UiFixture::new().with_points(256)),
    ];
    for (name, mut fixture) in scenes {
        fixture.snapshot.visible = name != "hidden";
        for _ in 0..WARM_UP_FRAMES {
            fixture.run();
        }
        let region = Region::new(&GLOBAL);
        for _ in 0..MEASURED_FRAMES {
            fixture.run();
        }
        let change = region.change();
        let per_frame = |count: usize| count as f64 / MEASURED_FRAMES as f64;
        println!(
            "{name}: {:.1} allocations, {:.1} reallocations, {:.0} bytes allocated per frame over {MEASURED_FRAMES} frames",
            per_frame(change.allocations),
            per_frame(change.reallocations),
            per_frame(change.bytes_allocated),
        );
    }
}

#[test]
fn bar_allocations_do_not_grow_with_points() {
    let few = allocations_per_frame(&mut UiFixture::new().with_points(3));
    let many = allocations_per_frame(&mut UiFixture::new().with_points(256));

    assert!(many <= few, "3 points: {few} allocations per frame, 256 points: {many}");
}
