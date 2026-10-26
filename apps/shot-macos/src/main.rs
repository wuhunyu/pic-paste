use eframe::egui;

#[derive(Default)]
struct ShotProbe { clicks: u64 }

impl eframe::App for ShotProbe {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("PicPaste - P0 engineering prototype");
            ui.label("UI smoke test only. Native capture and pinning are not implemented.");
            if ui.button("Test interaction").clicked() { self.clicks += 1; }
            ui.label(format!("Clicks: {}", self.clicks));
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([460.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native("PicPaste", options, Box::new(|_cc| Ok(Box::new(ShotProbe::default()))))
}
