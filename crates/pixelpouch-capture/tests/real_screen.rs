//! Tests against the real display. Ignored by default because CI runners
//! have no screen; run with `cargo test -p pixelpouch-capture -- --ignored --nocapture`.

use std::time::Instant;

use pixelpouch_capture::{CaptureBackend, CaptureOptions, default_backend, geometry};

#[test]
#[ignore = "needs a real display"]
fn enumerate_and_capture_every_monitor() {
    let backend = default_backend();
    let monitors = backend.monitors().expect("enumerate monitors");
    assert!(!monitors.is_empty());
    println!(
        "virtual desktop: {:?}",
        geometry::virtual_desktop(&monitors)
    );

    for m in &monitors {
        let start = Instant::now();
        let c = backend
            .capture_monitor(m.id, &CaptureOptions::default())
            .expect("capture monitor");
        let elapsed = start.elapsed();
        println!(
            "{} {:?} bounds={:?} pixels={:?} scale={} captured {:?} in {elapsed:?}",
            m.name,
            m.id,
            m.bounds,
            m.pixel_size,
            m.scale_factor,
            c.image.dimensions()
        );
        assert_eq!(
            c.image.dimensions(),
            (m.pixel_size.width, m.pixel_size.height)
        );
    }

    let desktop = geometry::virtual_desktop(&monitors);
    let start = Instant::now();
    let c = backend
        .capture_rect(desktop, &CaptureOptions::default())
        .expect("capture whole desktop");
    println!(
        "whole desktop {:?} in {:?}",
        c.image.dimensions(),
        start.elapsed()
    );

    let windows = backend.windows().expect("enumerate windows");
    for w in windows.iter().take(5) {
        println!(
            "window z={} {:?} {:?} ({})",
            w.z_order, w.bounds, w.title, w.app_name
        );
    }
}
