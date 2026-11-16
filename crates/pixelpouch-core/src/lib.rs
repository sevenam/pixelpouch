//! Shared types and configuration used across PixelPouch crates.

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
}

/// Convenience result alias using [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
