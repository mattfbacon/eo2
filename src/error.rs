use eframe::NativeOptions;
use egui::{Ui, ViewportCommand};

pub struct Stringed(pub String);

impl<E: std::error::Error> From<E> for Stringed {
	fn from(e: E) -> Self {
		Self(e.to_string())
	}
}

pub fn show(error: String) {
	eframe::run_native(
		"Error!",
		NativeOptions::default(),
		Box::new(|_cc| Ok(Box::new(App { error }))),
	)
	.unwrap();
}

struct App {
	error: String,
}

impl eframe::App for App {
	fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
		egui::Panel::bottom("ok-button-panel").show_inside(ui, |ui| {
			ui.vertical_centered(|ui| {
				if ui.button("Ok").clicked() {
					ui.send_viewport_cmd(ViewportCommand::Close);
				}
			});
		});

		egui::CentralPanel::default().show_inside(ui, |ui| {
			ui.heading("A fatal error occurred and the application will now exit.");
			ui.label(&self.error);
		});
	}
}
