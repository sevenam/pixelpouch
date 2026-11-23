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

    /// The platform capture API reported an error.
    #[error("screen capture failed: {0}")]
    Capture(String),

    /// No monitor with the given id is currently connected.
    #[error("monitor {0} not found")]
    MonitorNotFound(u32),

    /// The requested capture region is empty or does not overlap any monitor.
    #[error("capture region is empty or does not overlap any monitor")]
    RegionOutsideDesktop,

    /// The capture backend does not support the requested option.
    #[error("not supported by this capture backend: {0}")]
    Unsupported(&'static str),
}

/// Convenience result alias using [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
