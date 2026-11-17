//! Persistent user settings, stored as TOML in the platform config directory.
//!
//! Loading never fails hard: a missing file yields defaults, and an unreadable or corrupt file is
//! moved aside (`settings.toml.bak`) and replaced by defaults, so the app always starts.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{APP_NAME, Error, Result};

/// Current on-disk schema version. Bump when a change needs an explicit migration.
pub const SETTINGS_VERSION: u32 = 1;

const FILE_NAME: &str = "settings.toml";

/// Image format used when saving captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// Lossless PNG (default).
    #[default]
    Png,
    /// Lossy JPEG.
    Jpeg,
    /// WebP (lossless).
    Webp,
}

impl ImageFormat {
    /// All formats, in display order.
    pub const ALL: [Self; 3] = [Self::Png, Self::Jpeg, Self::Webp];

    /// Human-readable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Webp => "WebP",
        }
    }

    /// File extension without the leading dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
}

/// Global hotkey bindings, as accelerator strings (e.g. `"Ctrl+Shift+4"`).
///
/// An empty string means "unbound". Parsing and registration happen in the capture layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    /// Start a region capture.
    pub capture_region: String,
    /// Capture the window under the cursor.
    pub capture_window: String,
    /// Capture the full screen.
    pub capture_fullscreen: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            capture_region: "Ctrl+Shift+4".to_owned(),
            capture_window: "Ctrl+Shift+5".to_owned(),
            capture_fullscreen: "Ctrl+Shift+3".to_owned(),
        }
    }
}

/// What happens around a capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // independent on/off user options
pub struct CaptureOptions {
    /// Include the mouse cursor in captures.
    pub include_cursor: bool,
    /// Copy the capture to the clipboard after saving it.
    pub copy_to_clipboard: bool,
    /// Open the capture in the editor after saving it.
    pub open_editor: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            include_cursor: false,
            copy_to_clipboard: true,
            open_editor: false,
        }
    }
}

/// All user settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Schema version of the file this was loaded from.
    pub version: u32,
    /// Folder holding the screenshot library.
    pub library_dir: PathBuf,
    /// Format for newly saved captures.
    pub image_format: ImageFormat,
    /// Global hotkeys.
    pub hotkeys: Hotkeys,
    /// Launch PixelPouch when the user logs in.
    pub start_with_os: bool,
    /// Capture behaviour.
    pub capture: CaptureOptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            library_dir: default_library_dir(),
            image_format: ImageFormat::default(),
            hotkeys: Hotkeys::default(),
            start_with_os: false,
            capture: CaptureOptions::default(),
        }
    }
}

impl Settings {
    /// Parses settings from TOML, filling in defaults for missing fields and migrating old
    /// schema versions.
    pub fn from_toml(text: &str) -> Result<Self> {
        let mut settings: Self = toml::from_str(text)?;
        settings.migrate();
        Ok(settings)
    }

    /// Serialises settings to TOML.
    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Upgrades settings loaded from an older schema to [`SETTINGS_VERSION`].
    fn migrate(&mut self) {
        if self.version > SETTINGS_VERSION {
            tracing::warn!(
                found = self.version,
                supported = SETTINGS_VERSION,
                "settings file is from a newer PixelPouch; unknown fields are ignored"
            );
        }
        // No migrations yet: version 1 is the first schema.
        self.version = SETTINGS_VERSION;
    }
}

/// Default library folder: `<Pictures>/PixelPouch`, falling back to the home directory and
/// finally the current directory.
#[must_use]
pub fn default_library_dir() -> PathBuf {
    let base = directories::UserDirs::new().and_then(|dirs| {
        dirs.picture_dir()
            .map(Path::to_path_buf)
            .or_else(|| Some(dirs.home_dir().to_path_buf()))
    });
    base.unwrap_or_default().join(APP_NAME)
}

/// Loads and saves [`Settings`] at a fixed path.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// Store at the platform config directory (e.g. `%APPDATA%\PixelPouch\config\settings.toml`
    /// on Windows, `~/.config/pixelpouch/settings.toml` on Linux).
    pub fn platform() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("", "", APP_NAME).ok_or(Error::NoConfigDir)?;
        Ok(Self::at(dirs.config_dir().join(FILE_NAME)))
    }

    /// Store at an explicit file path.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Path of the settings file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads settings, falling back to defaults when the file is missing or corrupt.
    ///
    /// A corrupt file is renamed to `settings.toml.bak` so the user's data isn't lost and the
    /// next save doesn't overwrite it silently.
    #[must_use]
    pub fn load_or_default(&self) -> Settings {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(path = %self.path.display(), "no settings file, using defaults");
                return Settings::default();
            }
            Err(e) => {
                tracing::warn!(path = %self.path.display(), error = %e, "cannot read settings, using defaults");
                return Settings::default();
            }
        };

        match Settings::from_toml(&text) {
            Ok(settings) => settings,
            Err(e) => {
                let backup = self.backup_path();
                tracing::warn!(
                    path = %self.path.display(),
                    backup = %backup.display(),
                    error = %e,
                    "corrupt settings file, moved aside and using defaults"
                );
                if let Err(e) = fs::rename(&self.path, &backup) {
                    tracing::warn!(error = %e, "failed to back up corrupt settings file");
                }
                Settings::default()
            }
        }
    }

    /// Saves settings atomically (write to a temp file, then rename over the old one).
    pub fn save(&self, settings: &Settings) -> Result<()> {
        let text = settings.to_toml()?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("toml.bak")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = SettingsStore::at(dir.path().join("nested").join(FILE_NAME));
        (dir, store)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, store) = store();
        assert_eq!(store.load_or_default(), Settings::default());
    }

    #[test]
    fn round_trips_through_disk() {
        let (_dir, store) = store();
        let settings = Settings {
            library_dir: PathBuf::from("/tmp/shots"),
            image_format: ImageFormat::Jpeg,
            start_with_os: true,
            hotkeys: Hotkeys {
                capture_region: "Alt+R".to_owned(),
                ..Hotkeys::default()
            },
            capture: CaptureOptions {
                include_cursor: true,
                copy_to_clipboard: false,
                open_editor: true,
            },
            ..Settings::default()
        };
        store.save(&settings).expect("save");
        assert_eq!(store.load_or_default(), settings);
    }

    #[test]
    fn corrupt_file_falls_back_and_is_backed_up() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().expect("parent")).expect("mkdir");
        fs::write(store.path(), "this is = = not toml").expect("write");

        assert_eq!(store.load_or_default(), Settings::default());
        assert!(!store.path().exists());
        assert!(store.backup_path().exists());
    }

    #[test]
    fn wrong_types_count_as_corrupt() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().expect("parent")).expect("mkdir");
        fs::write(store.path(), "image_format = 42").expect("write");
        assert_eq!(store.load_or_default(), Settings::default());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let settings = Settings::from_toml("start_with_os = true\n[capture]\nopen_editor = true\n")
            .expect("parse");
        assert!(settings.start_with_os);
        assert!(settings.capture.open_editor);
        assert_eq!(
            settings.capture.copy_to_clipboard,
            CaptureOptions::default().copy_to_clipboard
        );
        assert_eq!(settings.hotkeys, Hotkeys::default());
        assert_eq!(settings.version, SETTINGS_VERSION);
    }

    #[test]
    fn newer_version_is_tolerated() {
        let settings = Settings::from_toml("version = 99\nfuture_option = 1\n").expect("parse");
        assert_eq!(settings.version, SETTINGS_VERSION);
    }

    #[test]
    fn default_library_dir_ends_with_app_name() {
        assert!(default_library_dir().ends_with(APP_NAME));
    }
}
