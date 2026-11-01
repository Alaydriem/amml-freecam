use serde::{Deserialize, Serialize};

const DEFAULT_FOLDER: &str = r"%USERPROFILE%\Pictures\Screenshots";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenshotConfig {
    // save folder; environment variables are expanded and the folder is created by the mod layer
    pub folder: String,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self { folder: DEFAULT_FOLDER.to_owned() }
    }
}

impl ScreenshotConfig {
    // returns a copy with a blank folder replaced by the default
    pub fn sanitized(self) -> Self {
        if self.folder.trim().is_empty() { Self::default() } else { self }
    }
}
