//! Shared types and configuration used across PixelPouch crates.

pub mod settings;

/// Human-readable application name.
pub const APP_NAME: &str = "PixelPouch";

/// Application version, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors shared by every PixelPouch library crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An underlying I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The settings file is not valid TOML or has the wrong shape.
    #[error("invalid settings file: {0}")]
    SettingsParse(#[from] toml::de::Error),

    /// Settings could not be serialised.
    #[error("cannot serialise settings: {0}")]
    SettingsSerialize(#[from] toml::ser::Error),

    /// The platform has no usable config directory (no home directory).
    #[error("no configuration directory available on this system")]
    NoConfigDir,
}

/// Convenience result alias using [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
