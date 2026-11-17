use eframe::egui;
use pixelpouch_core::settings::{Settings, SettingsStore};

use crate::settings_ui;

/// Top-level sections reachable from the navigation bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Library,
    Editor,
    Settings,
}

impl Section {
    const ALL: [Self; 3] = [Self::Library, Self::Editor, Self::Settings];

    fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Editor => "Editor",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug)]
pub struct PixelPouchApp {
    section: Section,
    /// Live settings; every part of the app reads from here, so edits apply immediately.
    settings: Settings,
    /// What was last written to disk, to detect changes.
    saved: Settings,
    /// `None` when the platform has no config directory; settings then live in memory only.
    store: Option<SettingsStore>,
    /// Last save error, shown on the settings screen.
    save_error: Option<String>,
}

impl PixelPouchApp {
    pub fn new(store: Option<SettingsStore>) -> Self {
        let settings = store
            .as_ref()
            .map_or_else(Settings::default, SettingsStore::load_or_default);
        tracing::info!(library = %settings.library_dir.display(), "settings loaded");
        Self {
            section: Section::default(),
            saved: settings.clone(),
            settings,
            store,
            save_error: None,
        }
    }

    /// Persists settings if they changed and applies side effects of the change.
    fn apply_changes(&mut self) {
        if self.settings == self.saved {
            return;
        }
        if self.settings.library_dir != self.saved.library_dir {
            tracing::info!(library = %self.settings.library_dir.display(), "library location changed");
        }
        let Some(store) = &self.store else {
            self.saved = self.settings.clone();
            return;
        };
        match store.save(&self.settings) {
            Ok(()) => {
                self.saved = self.settings.clone();
                self.save_error = None;
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to save settings");
                self.save_error = Some(e.to_string());
                // Don't retry every frame; the next edit will try again.
                self.saved = self.settings.clone();
            }
        }
    }

    fn library_ui(&self, ui: &mut egui::Ui) {
        ui.label("Library folder:");
        ui.monospace(self.settings.library_dir.display().to_string());
        ui.add_space(8.0);
        ui.label("Coming soon.");
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(store) = &self.store {
            ui.weak(format!("Saved to {}", store.path().display()));
        } else {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "No configuration folder is available; settings will not be kept after you quit.",
            );
        }
        if let Some(err) = &self.save_error {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Could not save settings: {err}"),
            );
        }
        settings_ui::show(ui, &mut self.settings);
    }
}

impl eframe::App for PixelPouchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(140.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading(pixelpouch_core::APP_NAME);
                ui.separator();
                for section in Section::ALL {
                    ui.selectable_value(&mut self.section, section, section.label());
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(self.section.label());
            match self.section {
                Section::Library => self.library_ui(ui),
                Section::Editor => {
                    ui.label("Coming soon.");
                }
                Section::Settings => self.settings_ui(ui),
            }
        });

        self.apply_changes();
    }
}
