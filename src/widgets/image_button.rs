use egui::{Response, Sense, TextureHandle, TextureId, Ui, Vec2, Widget, WidgetInfo, WidgetType};

/// Similar to `egui::widgets::ImageButton` but preserves the aspect ratio of the texture.
pub struct ImageButton {
	texture: TextureId,
	image_size_actual: Vec2,
	button_size: Vec2,
	selected: bool,
}

impl ImageButton {
	pub fn new(texture: &TextureHandle, button_size: Vec2) -> Self {
		Self {
			texture: texture.id(),
			image_size_actual: texture.size_vec2(),
			button_size,
			selected: false,
		}
	}

	pub fn selected(self, selected: bool) -> Self {
		Self { selected, ..self }
	}
}

impl Widget for ImageButton {
	fn ui(self, ui: &mut Ui) -> Response {
		let Self {
			texture,
			button_size,
			image_size_actual,
			selected,
		} = self;

		let padding = Vec2::splat(ui.spacing().button_padding.x);
		let (rect, response) = ui.allocate_exact_size(button_size, Sense::click());
		response.widget_info(|| WidgetInfo::new(WidgetType::ImageButton));

		if ui.is_rect_visible(rect) {
			let (rounding, fill, stroke) = if selected {
				let visuals = ui.visuals().selection;
				(egui::Rounding::none(), visuals.bg_fill, visuals.stroke)
			} else {
				let visuals = ui.style().interact(&response);
				(visuals.rounding, visuals.bg_fill, visuals.bg_stroke)
			};

			// Draw frame background (for transparent images):
			ui.painter().rect_filled(rect, rounding, fill);

			let available_rect = rect.shrink2(padding);
			super::Image::new(texture, image_size_actual).paint_at(ui, available_rect);

			// Draw frame outline:
			ui.painter().rect_stroke(rect, rounding, stroke);
		}

		response
	}
}
