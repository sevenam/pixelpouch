//! Integer geometry in virtual-desktop coordinates and the mapping from
//! desktop units to the physical pixels of a captured image.
//!
//! # Desktop units
//!
//! All monitor and window bounds are expressed in the platform's virtual
//! desktop coordinate space, which may extend into negative coordinates
//! (e.g. a monitor placed left of or above the primary one):
//!
//! - **Windows / X11**: physical pixels (the process must be per-monitor DPI
//!   aware on Windows, which the app is via winit).
//! - **macOS**: logical points; a Retina monitor has 2 pixels per point.
//!
//! [`MonitorInfo::pixel_size`] gives the size of a captured image of the whole
//! monitor, so the ratio between the two is the monitor's pixel density.

/// A point in virtual-desktop coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A width/height pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle; `x`/`y` is the top-left corner, right and
/// bottom edges are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Builds the rectangle spanned by two corner points, in any order.
    #[must_use]
    pub fn from_corners(a: Point, b: Point) -> Self {
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        Self::from_edges(i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1))
    }

    /// Exclusive right edge. Uses `i64` so it cannot overflow.
    #[must_use]
    pub fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    /// Exclusive bottom edge. Uses `i64` so it cannot overflow.
    #[must_use]
    pub fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    #[must_use]
    pub const fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    #[must_use]
    pub fn contains(&self, p: Point) -> bool {
        let (px, py) = (i64::from(p.x), i64::from(p.y));
        px >= i64::from(self.x)
            && px < self.right()
            && py >= i64::from(self.y)
            && py < self.bottom()
    }

    /// The overlapping area of two rectangles, or `None` if they do not overlap.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let x0 = i64::from(self.x.max(other.x));
        let y0 = i64::from(self.y.max(other.y));
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        (x1 > x0 && y1 > y0).then(|| Self::from_edges(x0, y0, x1, y1))
    }

    /// The smallest rectangle containing both.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self::from_edges(
            i64::from(self.x.min(other.x)),
            i64::from(self.y.min(other.y)),
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        )
    }

    /// Moves the rectangle so its coordinates are relative to `origin`.
    #[must_use]
    pub fn relative_to(&self, origin: Point) -> Self {
        Self::new(
            self.x.saturating_sub(origin.x),
            self.y.saturating_sub(origin.y),
            self.width,
            self.height,
        )
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from_edges(x0: i64, y0: i64, x1: i64, y1: i64) -> Self {
        let clamp = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let len = |a: i64, b: i64| (b - a).clamp(0, i64::from(u32::MAX)) as u32;
        Self::new(clamp(x0), clamp(y0), len(x0, x1), len(y0, y1))
    }
}

/// Opaque identifier of a monitor, stable while it stays connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonitorId(pub u32);

/// A connected monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id: MonitorId,
    /// Human-readable name, e.g. "DELL U2720Q".
    pub name: String,
    /// Bounds in virtual-desktop units (see the module docs).
    pub bounds: Rect,
    /// Size in physical pixels of a full capture of this monitor.
    pub pixel_size: Size,
    /// The OS UI scale factor (1.0 = 96 DPI on Windows, 2.0 = Retina).
    pub scale_factor: f32,
    pub is_primary: bool,
}

impl MonitorInfo {
    /// Physical pixels per desktop unit along each axis.
    #[must_use]
    pub fn pixel_density(&self) -> (f64, f64) {
        let ratio = |px: u32, units: u32| {
            if units == 0 {
                1.0
            } else {
                f64::from(px) / f64::from(units)
            }
        };
        (
            ratio(self.pixel_size.width, self.bounds.width),
            ratio(self.pixel_size.height, self.bounds.height),
        )
    }

    /// Maps a desktop rectangle to the pixel rectangle it covers in a full
    /// capture of this monitor, clipped to the monitor.
    ///
    /// Partially covered pixels at the edges are included, so the result
    /// always fully contains the requested area.
    #[must_use]
    pub fn desktop_to_pixels(&self, rect: &Rect) -> Option<Rect> {
        let clipped = self.bounds.intersection(rect)?;
        let local = clipped.relative_to(self.bounds.origin());
        let (dx, dy) = self.pixel_density();
        let (lx, ly) = (f64::from(local.x), f64::from(local.y));
        let x0 = (lx * dx).floor();
        let y0 = (ly * dy).floor();
        let x1 = ((lx + f64::from(local.width)) * dx).ceil();
        let y1 = ((ly + f64::from(local.height)) * dy).ceil();
        #[allow(clippy::cast_possible_truncation)]
        let px = Rect::from_edges(x0 as i64, y0 as i64, x1 as i64, y1 as i64);
        let full = Rect::new(0, 0, self.pixel_size.width, self.pixel_size.height);
        px.intersection(&full)
    }

    /// Maps a pixel position in a full capture of this monitor back to
    /// virtual-desktop coordinates.
    #[must_use]
    pub fn pixel_to_desktop(&self, px: Point) -> Point {
        let (dx, dy) = self.pixel_density();
        #[allow(clippy::cast_possible_truncation)]
        let map =
            |p: i32, d: f64, origin: i32| origin.saturating_add((f64::from(p) / d).floor() as i32);
        Point::new(map(px.x, dx, self.bounds.x), map(px.y, dy, self.bounds.y))
    }
}

/// The bounding box of all monitors (the virtual desktop).
#[must_use]
pub fn virtual_desktop(monitors: &[MonitorInfo]) -> Rect {
    monitors
        .iter()
        .fold(Rect::default(), |acc, m| acc.union(&m.bounds))
}

/// The monitor containing `p`, if any.
#[must_use]
pub fn monitor_at(monitors: &[MonitorInfo], p: Point) -> Option<&MonitorInfo> {
    monitors.iter().find(|m| m.bounds.contains(p))
}

/// Opaque identifier of a top-level window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

/// A top-level window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: String,
    /// Name of the owning application.
    pub app_name: String,
    pub pid: u32,
    /// Bounds in virtual-desktop units.
    pub bounds: Rect,
    /// Stacking order; higher values are closer to the viewer.
    pub z_order: i32,
    pub is_minimized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: u32, bounds: Rect, pixel_size: Size) -> MonitorInfo {
        MonitorInfo {
            id: MonitorId(id),
            name: format!("m{id}"),
            bounds,
            pixel_size,
            scale_factor: 1.0,
            is_primary: id == 0,
        }
    }

    #[test]
    fn intersection_and_union() {
        let a = Rect::new(-100, -50, 200, 100);
        let b = Rect::new(50, 0, 200, 200);
        assert_eq!(a.intersection(&b), Some(Rect::new(50, 0, 50, 50)));
        assert_eq!(a.union(&b), Rect::new(-100, -50, 350, 250));
        // Touching edges do not overlap.
        assert_eq!(a.intersection(&Rect::new(100, -50, 10, 10)), None);
        assert_eq!(Rect::default().union(&a), a);
    }

    #[test]
    fn contains_uses_exclusive_far_edges() {
        let r = Rect::new(-10, -10, 10, 10);
        assert!(r.contains(Point::new(-10, -10)));
        assert!(r.contains(Point::new(-1, -1)));
        assert!(!r.contains(Point::new(0, -1)));
    }

    #[test]
    fn from_corners_normalises() {
        let r = Rect::from_corners(Point::new(10, -5), Point::new(-20, 15));
        assert_eq!(r, Rect::new(-20, -5, 30, 20));
    }

    #[test]
    fn extreme_coordinates_do_not_overflow() {
        let r = Rect::new(i32::MAX - 1, i32::MAX - 1, u32::MAX, u32::MAX);
        assert_eq!(r.right(), i64::from(i32::MAX) - 1 + i64::from(u32::MAX));
        assert!(r.intersection(&Rect::new(0, 0, 10, 10)).is_none());
    }

    #[test]
    fn virtual_desktop_spans_negative_monitors() {
        let ms = [
            monitor(0, Rect::new(0, 0, 3840, 2160), Size::new(3840, 2160)),
            monitor(1, Rect::new(-1920, 400, 1920, 1080), Size::new(1920, 1080)),
            monitor(2, Rect::new(3840, -1440, 2560, 1440), Size::new(2560, 1440)),
        ];
        assert_eq!(virtual_desktop(&ms), Rect::new(-1920, -1440, 8320, 3600));
        assert_eq!(
            monitor_at(&ms, Point::new(-1, 400)).map(|m| m.id),
            Some(MonitorId(1))
        );
        assert_eq!(monitor_at(&ms, Point::new(-1, 399)), None);
        assert_eq!(
            monitor_at(&ms, Point::new(3840, -1)).map(|m| m.id),
            Some(MonitorId(2))
        );
    }

    #[test]
    fn desktop_to_pixels_unit_density() {
        let m = monitor(1, Rect::new(-1920, 0, 1920, 1080), Size::new(1920, 1080));
        assert_eq!(
            m.desktop_to_pixels(&Rect::new(-2000, 100, 200, 50)),
            Some(Rect::new(0, 100, 120, 50))
        );
        assert_eq!(m.desktop_to_pixels(&Rect::new(0, 0, 10, 10)), None);
    }

    #[test]
    fn desktop_to_pixels_retina() {
        // macOS-style: bounds in points, 2x pixels.
        let m = monitor(0, Rect::new(-1512, -982, 1512, 982), Size::new(3024, 1964));
        assert_eq!(m.pixel_density(), (2.0, 2.0));
        assert_eq!(
            m.desktop_to_pixels(&Rect::new(-1512, -982, 10, 5)),
            Some(Rect::new(0, 0, 20, 10))
        );
        assert_eq!(
            m.pixel_to_desktop(Point::new(21, 11)),
            Point::new(-1502, -977)
        );
    }

    #[test]
    fn desktop_to_pixels_fractional_density_covers_request() {
        // 1.5 pixels per unit: a 3-unit-wide area starting at unit 1 covers
        // pixels 1.5..6.0, so pixels 1..6 must be included.
        let m = monitor(0, Rect::new(0, 0, 100, 100), Size::new(150, 150));
        assert_eq!(
            m.desktop_to_pixels(&Rect::new(1, 1, 3, 3)),
            Some(Rect::new(1, 1, 5, 5))
        );
    }
}
