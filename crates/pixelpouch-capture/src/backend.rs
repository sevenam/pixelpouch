//! The platform-independent capture API.

use image::{RgbaImage, imageops};

use crate::geometry::{MonitorId, MonitorInfo, Rect, WindowInfo};
use pixelpouch_core::{Error, Result};

/// Options shared by all capture calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureOptions {
    /// Draw the mouse cursor into the image.
    pub include_cursor: bool,
}

/// A captured image together with the desktop area it shows.
#[derive(Debug, Clone)]
pub struct Capture {
    /// Pixels in RGBA order. Areas of `bounds` not covered by any monitor
    /// are fully transparent.
    pub image: RgbaImage,
    /// The captured area in virtual-desktop units.
    pub bounds: Rect,
}

/// A source of screen pixels.
///
/// Backends implement the primitive operations; capturing a monitor by id
/// and capturing arbitrary rectangles spanning several monitors (with
/// different pixel densities) are provided on top of them.
pub trait CaptureBackend {
    /// All connected monitors.
    fn monitors(&self) -> Result<Vec<MonitorInfo>>;

    /// All top-level windows, topmost first.
    fn windows(&self) -> Result<Vec<WindowInfo>>;

    /// Captures `region` of `monitor` at the monitor's native resolution.
    ///
    /// `region` is in desktop units relative to the monitor's top-left
    /// corner and lies entirely within the monitor.
    fn capture_monitor_region(
        &self,
        monitor: &MonitorInfo,
        region: Rect,
        options: &CaptureOptions,
    ) -> Result<RgbaImage>;

    /// Captures a whole monitor at its native resolution.
    fn capture_monitor(&self, id: MonitorId, options: &CaptureOptions) -> Result<Capture> {
        let monitors = self.monitors()?;
        let monitor = monitors
            .iter()
            .find(|m| m.id == id)
            .ok_or(Error::MonitorNotFound(id.0))?;
        let local = Rect::new(0, 0, monitor.bounds.width, monitor.bounds.height);
        let image = self.capture_monitor_region(monitor, local, options)?;
        Ok(Capture {
            image,
            bounds: monitor.bounds,
        })
    }

    /// Captures an arbitrary rectangle in virtual-desktop units, which may
    /// span several monitors and include negative coordinates.
    ///
    /// The output uses the highest pixel density of the monitors involved,
    /// so nothing is downscaled; lower-density parts are upscaled to match.
    /// When all monitors involved share the same density (always the case
    /// on Windows and X11), pixels are copied exactly.
    fn capture_rect(&self, rect: Rect, options: &CaptureOptions) -> Result<Capture> {
        let monitors = self.monitors()?;
        compose(&monitors, rect, |m, local| {
            self.capture_monitor_region(m, local, options)
        })
    }
}

/// Assembles a capture of `rect` from per-monitor captures.
pub(crate) fn compose(
    monitors: &[MonitorInfo],
    rect: Rect,
    mut capture: impl FnMut(&MonitorInfo, Rect) -> Result<RgbaImage>,
) -> Result<Capture> {
    let pieces: Vec<(&MonitorInfo, Rect)> = monitors
        .iter()
        .filter_map(|m| m.bounds.intersection(&rect).map(|piece| (m, piece)))
        .collect();
    if pieces.is_empty() {
        return Err(Error::RegionOutsideDesktop);
    }

    let (dx, dy) = pieces.iter().fold((0.0_f64, 0.0_f64), |(dx, dy), (m, _)| {
        let (mx, my) = m.pixel_density();
        (dx.max(mx), dy.max(my))
    });

    // Fast path: the request lies within a single monitor.
    if let [(m, piece)] = pieces.as_slice()
        && *piece == rect
    {
        let image = capture(m, piece.relative_to(m.bounds.origin()))?;
        return Ok(Capture {
            image,
            bounds: rect,
        });
    }

    let out_w = scale_len(rect.width, dx);
    let out_h = scale_len(rect.height, dy);
    let mut out = RgbaImage::new(out_w, out_h);

    for (m, piece) in pieces {
        let mut image = capture(m, piece.relative_to(m.bounds.origin()))?;
        let offset = piece.relative_to(rect.origin());
        let x0 = scale_pos(offset.x, dx);
        let y0 = scale_pos(offset.y, dy);
        let x1 = scale_pos_end(offset.x, offset.width, dx).min(out_w);
        let y1 = scale_pos_end(offset.y, offset.height, dy).min(out_h);
        let (tw, th) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
        if tw == 0 || th == 0 {
            continue;
        }
        if image.dimensions() != (tw, th) {
            tracing::debug!(
                monitor = m.id.0,
                from = ?image.dimensions(),
                to = ?(tw, th),
                "resampling monitor piece to output density"
            );
            image = imageops::resize(&image, tw, th, imageops::FilterType::Triangle);
        }
        imageops::replace(&mut out, &image, i64::from(x0), i64::from(y0));
    }

    Ok(Capture {
        image: out,
        bounds: rect,
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scale_len(len: u32, density: f64) -> u32 {
    (f64::from(len) * density)
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Offsets are always non-negative here because pieces lie inside the request.
#[allow(clippy::cast_sign_loss)]
fn scale_pos(offset: i32, density: f64) -> u32 {
    scale_len(offset.max(0) as u32, density)
}

#[allow(clippy::cast_sign_loss)]
fn scale_pos_end(offset: i32, len: u32, density: f64) -> u32 {
    scale_len((offset.max(0) as u32).saturating_add(len), density)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Size};
    use image::Rgba;

    /// A fake desktop where every pixel encodes its monitor id and its
    /// desktop-unit coordinates, so composition can be checked exactly.
    struct FakeBackend {
        monitors: Vec<MonitorInfo>,
    }

    fn pixel_for(m: &MonitorInfo, desktop: Point) -> Rgba<u8> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let b = |v: i32| (v.rem_euclid(256)) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let id = m.id.0 as u8;
        Rgba([id, b(desktop.x), b(desktop.y), 255])
    }

    impl CaptureBackend for FakeBackend {
        fn monitors(&self) -> Result<Vec<MonitorInfo>> {
            Ok(self.monitors.clone())
        }

        fn windows(&self) -> Result<Vec<WindowInfo>> {
            Ok(Vec::new())
        }

        fn capture_monitor_region(
            &self,
            monitor: &MonitorInfo,
            region: Rect,
            _options: &CaptureOptions,
        ) -> Result<RgbaImage> {
            let local_full = Rect::new(0, 0, monitor.bounds.width, monitor.bounds.height);
            assert_eq!(
                local_full.intersection(&region),
                Some(region),
                "region must lie within the monitor"
            );
            let abs = Rect::new(
                monitor.bounds.x + region.x,
                monitor.bounds.y + region.y,
                region.width,
                region.height,
            );
            let px = monitor.desktop_to_pixels(&abs).expect("inside monitor");
            Ok(RgbaImage::from_fn(px.width, px.height, |x, y| {
                #[allow(clippy::cast_possible_wrap)]
                let p = Point::new(px.x + x as i32, px.y + y as i32);
                pixel_for(monitor, monitor.pixel_to_desktop(p))
            }))
        }
    }

    fn monitor(id: u32, bounds: Rect, density: u32) -> MonitorInfo {
        MonitorInfo {
            id: MonitorId(id),
            name: format!("m{id}"),
            bounds,
            pixel_size: Size::new(bounds.width * density, bounds.height * density),
            #[allow(clippy::cast_precision_loss)]
            scale_factor: density as f32,
            is_primary: id == 1,
        }
    }

    /// Windows-style layout: primary at the origin, a second monitor to the
    /// left with a negative x and offset vertically.
    fn windows_layout() -> FakeBackend {
        FakeBackend {
            monitors: vec![
                monitor(1, Rect::new(0, 0, 400, 300), 1),
                monitor(2, Rect::new(-200, -50, 200, 150), 1),
            ],
        }
    }

    #[test]
    fn capture_monitor_by_id() {
        let b = windows_layout();
        let c = b
            .capture_monitor(MonitorId(2), &CaptureOptions::default())
            .expect("capture");
        assert_eq!(c.bounds, Rect::new(-200, -50, 200, 150));
        assert_eq!(c.image.dimensions(), (200, 150));
        assert_eq!(
            *c.image.get_pixel(0, 0),
            pixel_for(&b.monitors[1], Point::new(-200, -50))
        );
        assert!(matches!(
            b.capture_monitor(MonitorId(9), &CaptureOptions::default()),
            Err(Error::MonitorNotFound(9))
        ));
    }

    #[test]
    fn rect_spanning_monitors_is_pixel_exact() {
        let b = windows_layout();
        let rect = Rect::new(-10, -60, 30, 80);
        let c = b
            .capture_rect(rect, &CaptureOptions::default())
            .expect("capture");
        assert_eq!(c.image.dimensions(), (30, 80));
        for (x, y, px) in c.image.enumerate_pixels() {
            #[allow(clippy::cast_possible_wrap)]
            let p = Point::new(rect.x + x as i32, rect.y + y as i32);
            let expected = b
                .monitors
                .iter()
                .find(|m| m.bounds.contains(p))
                .map_or(Rgba([0, 0, 0, 0]), |m| pixel_for(m, p));
            assert_eq!(*px, expected, "pixel ({x}, {y}) = desktop {p:?}");
        }
    }

    #[test]
    fn mixed_density_uses_highest_density() {
        // macOS-style: a Retina monitor at 2x next to a 1x monitor above it
        // with negative y.
        let b = FakeBackend {
            monitors: vec![
                monitor(1, Rect::new(0, 0, 100, 60), 2),
                monitor(2, Rect::new(-20, -40, 160, 40), 1),
            ],
        };
        let rect = Rect::new(10, -10, 20, 20);
        let c = b
            .capture_rect(rect, &CaptureOptions::default())
            .expect("capture");
        assert_eq!(c.image.dimensions(), (40, 40));
        // Bottom half comes from the 2x monitor unchanged.
        assert_eq!(
            *c.image.get_pixel(0, 20),
            pixel_for(&b.monitors[0], Point::new(10, 0))
        );
        assert_eq!(
            *c.image.get_pixel(39, 39),
            pixel_for(&b.monitors[0], Point::new(29, 9))
        );
        // Top half is the 1x monitor upscaled; its corner pixel survives.
        assert_eq!(c.image.get_pixel(0, 0)[0], 2);
        assert_eq!(c.image.get_pixel(39, 19)[0], 2);
    }

    #[test]
    fn rect_outside_desktop_is_an_error() {
        let b = windows_layout();
        let res = b.capture_rect(Rect::new(1000, 1000, 10, 10), &CaptureOptions::default());
        assert!(matches!(res, Err(Error::RegionOutsideDesktop)));
        let res = b.capture_rect(Rect::new(0, 0, 0, 10), &CaptureOptions::default());
        assert!(matches!(res, Err(Error::RegionOutsideDesktop)));
    }

    #[test]
    fn gaps_between_monitors_are_transparent() {
        let b = windows_layout();
        // (-100, 200) is below monitor 2 and left of monitor 1.
        let c = b
            .capture_rect(Rect::new(-100, 90, 200, 120), &CaptureOptions::default())
            .expect("capture");
        assert_eq!(*c.image.get_pixel(0, 110), Rgba([0, 0, 0, 0]));
        assert_eq!(c.image.get_pixel(150, 110)[0], 1);
    }
}
