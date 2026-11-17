//! PixelPouch desktop application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod settings_ui;

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    init_logging();
    tracing::info!(version = pixelpouch_core::VERSION, "starting");

    let store = match pixelpouch_core::settings::SettingsStore::platform() {
        Ok(store) => Some(store),
        Err(e) => {
            tracing::warn!(error = %e, "settings will not be persisted");
            None
        }
    };

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(pixelpouch_core::APP_NAME)
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([480.0, 320.0]),
        wgpu_options: wgpu_options(),
        ..Default::default()
    };

    eframe::run_native(
        pixelpouch_core::APP_NAME,
        options,
        Box::new(move |_cc| Ok(Box::new(app::PixelPouchApp::new(store)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
    .context("failed to run the application window")
}

/// wgpu configuration with a single default backend per platform.
///
/// Probing every backend (Vulkan, DX12, GL) at startup costs ~100 ms on Windows, so we pick
/// one up front. `WGPU_BACKEND` (e.g. `vulkan`, `dx12`, `gl`) still overrides the choice.
fn wgpu_options() -> eframe::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu::Backends;

    let preferred = if cfg!(windows) {
        Backends::DX12
    } else {
        Backends::PRIMARY | Backends::GL
    };

    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = Backends::from_env().unwrap_or(preferred);

    eframe::WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

/// Logs to stderr; level is controlled by `RUST_LOG` (default `info`).
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
