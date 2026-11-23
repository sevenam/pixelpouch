//! [`CaptureBackend`] implemented on top of the cross-platform `xcap` crate
//! (GDI on Windows, `CoreGraphics` on macOS, XCB on X11).

use image::RgbaImage;
use pixelpouch_core::{Error, Result};

use crate::backend::{CaptureBackend, CaptureOptions};
use crate::geometry::{MonitorId, MonitorInfo, Rect, Size, WindowId, WindowInfo};

/// Capture backend using `xcap`.
#[derive(Debug, Default, Clone, Copy)]
pub struct XcapBackend;

impl XcapBackend {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn find_monitor(id: MonitorId) -> Result<xcap::Monitor> {
        xcap::Monitor::all()
            .map_err(capture_err)?
            .into_iter()
            .find(|m| m.id().is_ok_and(|mid| mid == id.0))
            .ok_or(Error::MonitorNotFound(id.0))
    }
}

#[allow(clippy::needless_pass_by_value)]
fn capture_err(e: xcap::XCapError) -> Error {
    Error::Capture(e.to_string())
}

fn monitor_info(m: &xcap::Monitor) -> xcap::XCapResult<MonitorInfo> {
    let bounds = Rect::new(m.x()?, m.y()?, m.width()?, m.height()?);
    let scale_factor = m.scale_factor()?;
    Ok(MonitorInfo {
        id: MonitorId(m.id()?),
        name: m.friendly_name().or_else(|_| m.name())?,
        bounds,
        pixel_size: pixel_size(bounds, scale_factor),
        scale_factor,
        is_primary: m.is_primary()?,
    })
}

/// On macOS bounds are in points and captures come back in pixels.
#[cfg(target_os = "macos")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pixel_size(bounds: Rect, scale_factor: f32) -> Size {
    let scale = |v: u32| (f64::from(v) * f64::from(scale_factor)).round() as u32;
    Size::new(scale(bounds.width), scale(bounds.height))
}

/// Elsewhere bounds are already physical pixels.
#[cfg(not(target_os = "macos"))]
fn pixel_size(bounds: Rect, _scale_factor: f32) -> Size {
    bounds.size()
}

fn window_info(w: &xcap::Window) -> xcap::XCapResult<WindowInfo> {
    Ok(WindowInfo {
        id: WindowId(w.id()?),
        title: w.title()?,
        app_name: w.app_name()?,
        pid: w.pid()?,
        bounds: Rect::new(w.x()?, w.y()?, w.width()?, w.height()?),
        z_order: w.z()?,
        is_minimized: w.is_minimized()?,
    })
}

impl CaptureBackend for XcapBackend {
    fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        xcap::Monitor::all()
            .map_err(capture_err)?
            .iter()
            .map(|m| monitor_info(m).map_err(capture_err))
            .collect()
    }

    fn windows(&self) -> Result<Vec<WindowInfo>> {
        let mut windows: Vec<WindowInfo> = xcap::Window::all()
            .map_err(capture_err)?
            .iter()
            // Windows can close while we enumerate; skip those that vanish.
            .filter_map(|w| {
                window_info(w)
                    .inspect_err(|e| tracing::debug!("skipping window: {e}"))
                    .ok()
            })
            .filter(|w| !w.bounds.is_empty())
            .collect();
        windows.sort_by_key(|w| std::cmp::Reverse(w.z_order));
        Ok(windows)
    }

    fn capture_monitor_region(
        &self,
        monitor: &MonitorInfo,
        region: Rect,
        options: &CaptureOptions,
    ) -> Result<RgbaImage> {
        if options.include_cursor {
            return Err(Error::Unsupported("cursor capture"));
        }
        let (Ok(x), Ok(y)) = (u32::try_from(region.x), u32::try_from(region.y)) else {
            return Err(Error::RegionOutsideDesktop);
        };
        let m = Self::find_monitor(monitor.id)?;
        let full = x == 0
            && y == 0
            && region.width == monitor.bounds.width
            && region.height == monitor.bounds.height;
        let image = if full {
            m.capture_image()
        } else {
            m.capture_region(x, y, region.width, region.height)
        };
        image.map_err(capture_err)
    }
}
