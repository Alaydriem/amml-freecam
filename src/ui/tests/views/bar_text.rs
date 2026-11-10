use crate::support::ui_fixture::{FrameResult, UiFixture};

// the position and colour of every vertex; the atlas places glyphs by history, so texture coordinates are left out
fn picture(out: &FrameResult) -> Vec<(f32, f32, [u8; 4])> {
    out.vertices.iter().map(|v| (v.x, v.y, v.rgba)).collect()
}

fn settled(mut fixture: UiFixture) -> UiFixture {
    fixture.settle();
    fixture
}

// new values for every bar read-out
fn change_values(fixture: &mut UiFixture) {
    fixture.snapshot.speed = 12.5;
    fixture.snapshot.playhead_seconds = 61.25;
    fixture.snapshot.total_seconds = 90.0;
    fixture.snapshot.point_count = 2;
}

#[test]
fn changed_read_outs_match_a_fresh_layout() {
    for selection in [None, Some(0)] {
        let mut kept = UiFixture::new().with_path();
        kept.snapshot.selection = selection.map(|_| 2);
        let mut kept = settled(kept);
        change_values(&mut kept);
        kept.snapshot.selection = selection;
        kept.settle();
        let mut fresh = UiFixture::new().with_path();
        change_values(&mut fresh);
        fresh.snapshot.selection = selection;
        let mut fresh = settled(fresh);

        assert_eq!(picture(kept.frame(&[])), picture(fresh.frame(&[])), "selection {selection:?}");
    }
}

#[test]
fn scale_change_lays_the_text_out_again() {
    let mut kept = settled(UiFixture::new().with_path());
    kept.pixels_per_point = 1.5;
    kept.settle();
    let mut fresh = UiFixture::new().with_path();
    fresh.pixels_per_point = 1.5;
    let mut fresh = settled(fresh);

    assert_eq!(picture(kept.frame(&[])), picture(fresh.frame(&[])));
}

#[test]
fn new_context_lays_the_text_out_again() {
    // the first frame in a new context fills a new atlas; a galley kept from the old one would point at other glyphs
    let mut kept = settled(UiFixture::new().with_path());
    kept.new_context();
    kept.ui.textures_lost();
    let mut fresh = UiFixture::new().with_path();
    fresh.time = kept.time;

    let kept_out = picture(kept.frame(&[]));
    let fresh_out = picture(fresh.frame(&[]));

    assert_eq!(kept_out, fresh_out);
}
