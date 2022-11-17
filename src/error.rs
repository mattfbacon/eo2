use eframe::NativeOptions;
use egui::Context;

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
		Box::new(|_cc| Box::new(App { error })),
	);
}

struct App {
	error: String,
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		egui::TopBottomPanel::bottom("ok-button-panel").show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				if ui.button("Ok").clicked() {
					frame.close();
				}
			});
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("A fatal error occurred and the application will now exit.");
			ui.label(&self.error);
		});
	}
}
