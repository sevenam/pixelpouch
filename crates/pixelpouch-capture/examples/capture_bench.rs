//! Times repeated full-monitor captures: `cargo run -p pixelpouch-capture --release --example capture_bench`.

use std::time::{Duration, Instant};

use pixelpouch_capture::{CaptureBackend, CaptureOptions, default_backend};

fn main() -> pixelpouch_capture::Result<()> {
    let backend = default_backend();
    for m in backend.monitors()? {
        let mut times: Vec<Duration> = (0..10)
            .map(|_| {
                let start = Instant::now();
                backend.capture_monitor(m.id, &CaptureOptions::default())?;
                Ok(start.elapsed())
            })
            .collect::<pixelpouch_capture::Result<_>>()?;
        times.sort();
        println!(
            "{} {}x{} px: min {:?}, median {:?}, max {:?}",
            m.name,
            m.pixel_size.width,
            m.pixel_size.height,
            times[0],
            times[times.len() / 2],
            times[times.len() - 1]
        );
    }
    Ok(())
}
