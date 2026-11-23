//! PixelPouch capture subsystem.
//!
//! [`CaptureBackend`] is the platform abstraction for grabbing pixels from
//! the screen: enumerate monitors and windows, capture a monitor, or capture
//! an arbitrary rectangle of the virtual desktop. [`default_backend`] returns
//! the implementation for the current platform.
//!
//! Coordinates are explained in [`geometry`].

pub mod backend;
pub mod geometry;
mod xcap_backend;

pub use backend::{Capture, CaptureBackend, CaptureOptions};
pub use geometry::{MonitorId, MonitorInfo, Point, Rect, Size, WindowId, WindowInfo};
pub use image::RgbaImage;
pub use pixelpouch_core::{Error, Result};
pub use xcap_backend::XcapBackend;

/// The capture backend for the current platform.
#[must_use]
pub fn default_backend() -> impl CaptureBackend {
    XcapBackend::new()
}
