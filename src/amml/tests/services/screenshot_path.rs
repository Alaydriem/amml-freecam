use std::path::{Path, PathBuf};

use alaydriem_freecam::services::ScreenshotPath;

fn vars(name: &str) -> Option<String> {
    match name {
        "USERPROFILE" => Some(r"C:\Users\pat".to_owned()),
        "SHOTS" => Some("D:/shots/".to_owned()),
        _ => None,
    }
}

#[test]
fn expand_replaces_set_variables_and_keeps_unknown_ones() {
    assert_eq!(
        ScreenshotPath::expand(r"%USERPROFILE%\Pictures\Screenshots", vars),
        r"C:\Users\pat\Pictures\Screenshots"
    );
    assert_eq!(ScreenshotPath::expand(r"%NOPE%\x", vars), r"%NOPE%\x");
    assert_eq!(ScreenshotPath::expand("%SHOTS%freecam/", vars), r"D:\shots\freecam");
    assert_eq!(ScreenshotPath::expand("50% done", vars), "50% done");
}

#[test]
fn relative_or_blank_folder_is_refused() {
    assert!(ScreenshotPath::new("shots", vars).is_none());
    assert!(ScreenshotPath::new("   ", vars).is_none());
    assert!(ScreenshotPath::new(r"%USERPROFILE%\Pictures", vars).is_some());
}

#[test]
fn next_creates_the_folder_and_names_the_file_by_time() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("a").join("b");
    let shots = ScreenshotPath::new(folder.to_str().unwrap(), vars).unwrap();

    let path = shots.next("20261021-101500").unwrap();

    assert!(folder.is_dir());
    assert_eq!(path, folder.join("freecam-20261021-101500.png"));
}

#[test]
fn next_adds_a_counter_when_the_name_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let shots = ScreenshotPath::new(dir.path().to_str().unwrap(), vars).unwrap();
    let taken = |name: &str| std::fs::write(dir.path().join(name), b"png").unwrap();
    taken("freecam-20261021-101500.png");
    taken("freecam-20261021-101500-2.png");

    let path: PathBuf = shots.next("20261021-101500").unwrap();

    assert_eq!(path.file_name().map(Path::new), Some(Path::new("freecam-20261021-101500-3.png")));
}
