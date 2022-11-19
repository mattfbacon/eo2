use egui::{Align, Grid, InnerResponse, Layout, Ui, WidgetText};

pub struct KeyValue(Grid);

pub struct Rows<'a>(&'a mut Ui);

impl Rows<'_> {
	pub fn row<R>(
		&mut self,
		key: impl Into<WidgetText>,
		value: impl FnOnce(&mut Ui) -> R,
	) -> InnerResponse<R> {
		let ui = &mut *self.0;

		ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
			ui.label(key);
		});

		let response = ui.with_layout(Layout::left_to_right(Align::Center), value);

		ui.end_row();

		response
	}

	pub fn separator(&mut self) {
		let ui = &mut *self.0;

		let spacing = 6.0;
		let available_space = ui.available_size_before_wrap();
		let size = egui::vec2(available_space.x, spacing);

		let (rect, response) = ui.allocate_at_least(size, egui::Sense::hover());

		if ui.is_rect_visible(response.rect) {
			let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
			// override to span the full grid
			let x_range = ui.max_rect().x_range();
			ui.painter().hline(x_range, rect.center().y, stroke);
		}

		ui.end_row();
	}

	pub fn sub<R>(
		&mut self,
		id: &str,
		key: impl Into<WidgetText>,
		sub: impl FnOnce(Rows<'_>) -> R,
	) -> InnerResponse<InnerResponse<R>> {
		self.row(key, |ui| KeyValue::new(id).show(ui, sub))
	}
}

impl KeyValue {
	pub fn new(id: &str) -> Self {
		Self(Grid::new(id).num_columns(2))
	}

	pub fn show<R>(self, ui: &mut Ui, show: impl FnOnce(Rows<'_>) -> R) -> InnerResponse<R> {
		self.0.show(ui, |ui| show(Rows(ui)))
	}
}
