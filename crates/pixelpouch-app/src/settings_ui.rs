//! The Settings screen.

use eframe::egui;
use pixelpouch_core::settings::{ImageFormat, Settings, default_library_dir};

/// Draws the settings form, editing `settings` in place.
pub fn show(ui: &mut egui::Ui, settings: &mut Settings) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Library", |ui| library(ui, settings));
        section(ui, "Captures", |ui| captures(ui, settings));
        section(ui, "Hotkeys", |ui| hotkeys(ui, settings));
        section(ui, "System", |ui| {
            ui.checkbox(
                &mut settings.start_with_os,
                "Start PixelPouch when I log in",
            );
        });
    });
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    ui.strong(title);
    ui.add_space(4.0);
    ui.indent(title, body);
    ui.add_space(4.0);
    ui.separator();
}

fn library(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Screenshots are saved in:");
    ui.horizontal(|ui| {
        ui.monospace(settings.library_dir.display().to_string());
    });
    ui.horizontal(|ui| {
        if ui.button("Choose folder…").clicked()
            && let Some(dir) = rfd::FileDialog::new()
                .set_title("Choose library folder")
                .set_directory(&settings.library_dir)
                .pick_folder()
        {
            settings.library_dir = dir;
        }
        let default_dir = default_library_dir();
        if ui
            .add_enabled(
                settings.library_dir != default_dir,
                egui::Button::new("Reset to default"),
            )
            .clicked()
        {
            settings.library_dir = default_dir;
        }
    });
}

fn captures(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.horizontal(|ui| {
        ui.label("Image format:");
        egui::ComboBox::from_id_salt("image_format")
            .selected_text(settings.image_format.label())
            .show_ui(ui, |ui| {
                for format in ImageFormat::ALL {
                    ui.selectable_value(&mut settings.image_format, format, format.label());
                }
            });
    });
    let capture = &mut settings.capture;
    ui.checkbox(&mut capture.include_cursor, "Include the mouse cursor");
    ui.checkbox(
        &mut capture.copy_to_clipboard,
        "Copy to clipboard after capture",
    );
    ui.checkbox(&mut capture.open_editor, "Open in editor after capture");
}

fn hotkeys(ui: &mut egui::Ui, settings: &mut Settings) {
    let keys = &mut settings.hotkeys;
    egui::Grid::new("hotkeys")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Capture region", &mut keys.capture_region),
                ("Capture window", &mut keys.capture_window),
                ("Capture full screen", &mut keys.capture_fullscreen),
            ] {
                ui.label(label);
                ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text("Unbound")
                        .desired_width(160.0),
                );
                ui.end_row();
            }
        });
    ui.weak("Use a combination like Ctrl+Shift+4. Leave empty to disable.");
}
