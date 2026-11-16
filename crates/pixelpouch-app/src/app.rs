use eframe::egui;

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

#[derive(Debug, Default)]
pub struct PixelPouchApp {
    section: Section,
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
            ui.label("Coming soon.");
        });
    }
}
