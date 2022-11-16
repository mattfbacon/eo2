use egui::{Align, Grid, InnerResponse, Layout, Ui, WidgetText};

pub struct KeyValue(Grid);

pub struct Rows<'a>(&'a mut Ui);

impl Rows<'_> {
	pub fn row<R>(
		&mut self,
		key: impl Into<WidgetText>,
		value: impl FnOnce(&mut Ui) -> R,
	) -> InnerResponse<R> {
		self
			.0
			.with_layout(Layout::right_to_left(Align::Center), |ui| {
				ui.label(key);
			});
		let response = self
			.0
			.with_layout(Layout::left_to_right(Align::Center), value);
		self.0.end_row();
		response
	}
}

impl KeyValue {
	pub fn new(id: &'static str) -> Self {
		Self(Grid::new(id).num_columns(2))
	}

	pub fn show<R>(self, ui: &mut Ui, show: impl FnOnce(Rows<'_>) -> R) -> InnerResponse<R> {
		self.0.show(ui, |ui| show(Rows(ui)))
	}
}
