use std::path::{Path, PathBuf};

// where screenshots go: `screenshot.folder` with its %VARIABLES% expanded, made if missing, and one file name per shot
pub struct ScreenshotPath {
    folder: PathBuf,
}

impl ScreenshotPath {
    // names tried for one second before the shot is refused
    pub const MAX_ATTEMPTS: u32 = 99;

    // the folder of `configured` after `var` expanded it; None when it is blank or not an absolute path
    pub fn new(configured: &str, var: impl Fn(&str) -> Option<String>) -> Option<ScreenshotPath> {
        let expanded = Self::expand(configured.trim(), var);
        let folder = PathBuf::from(expanded);
        folder.is_absolute().then_some(ScreenshotPath { folder })
    }

    // replaces every %NAME% that `var` knows, leaves the others as they are, turns `/` into `\` and drops trailing
    // separators
    pub fn expand(configured: &str, var: impl Fn(&str) -> Option<String>) -> String {
        let mut out = String::with_capacity(configured.len());
        let mut rest = configured;
        while let Some(start) = rest.find('%') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('%').and_then(|end| var(&after[..end]).map(|value| (end, value))) {
                Some((end, value)) => {
                    out.push_str(&value);
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        let mut out = out.replace('/', "\\");
        while out.len() > 1 && out.ends_with('\\') {
            out.pop();
        }
        out
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    // the path of the next shot taken at `stamp` (`YYYYMMDD-HHMMSS`): freecam-<stamp>.png, or freecam-<stamp>-N.png
    // when that name is taken. Makes the folder first; None when it cannot be made or every name is taken
    pub fn next(&self, stamp: &str) -> Option<PathBuf> {
        if let Err(error) = std::fs::create_dir_all(&self.folder) {
            curia::warn!("screenshot folder not made", { folder: self.folder.display().to_string(), error: error.to_string() });
            return None;
        }
        (1..=Self::MAX_ATTEMPTS)
            .map(|attempt| self.folder.join(Self::file_name(stamp, attempt)))
            .find(|path| !path.exists())
    }

    fn file_name(stamp: &str, attempt: u32) -> String {
        if attempt <= 1 { format!("freecam-{stamp}.png") } else { format!("freecam-{stamp}-{attempt}.png") }
    }
}
