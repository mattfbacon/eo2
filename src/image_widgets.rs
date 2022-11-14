use egui::{
	Rect, Response, Sense, TextureHandle, TextureId, Ui, Vec2, Widget, WidgetInfo, WidgetType,
};

use crate::image_size;

/// Similar to `egui::widgets::Image` but preserves the aspect ratio of the texture.
pub struct Image {
	texture: TextureId,
	actual_size: Vec2,
	sense: Sense,
}

impl Image {
	pub fn new(texture: TextureId, size: Vec2) -> Self {
		Self {
			texture,
			actual_size: size,
			sense: Sense::hover(),
		}
	}

	pub fn for_texture(texture: &TextureHandle) -> Self {
		Self::new(texture.id(), texture.size_vec2())
	}

	pub fn sense(self, sense: Sense) -> Self {
		Self { sense, ..self }
	}

	/// Returns the actual rect that the image filled
	pub fn paint_at(self, ui: &mut Ui, available_rect: Rect) -> Rect {
		let available_size = available_rect.size();
		let scaled_size = image_size(self.actual_size, available_size);
		let image_rect = ui
			.layout()
			.align_size_within_rect(scaled_size, available_rect);
		egui::widgets::Image::new(self.texture, scaled_size).paint_at(ui, image_rect);
		image_rect
	}
}

impl Widget for Image {
	fn ui(self, ui: &mut Ui) -> Response {
		let (id, space) = ui.allocate_space(ui.available_size());
		let sense = self.sense;
		let image_rect = self.paint_at(ui, space);
		ui.interact(image_rect, id, sense)
	}
}

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
			Image::new(texture, image_size_actual).paint_at(ui, available_rect);

			// Draw frame outline:
			ui.painter().rect_stroke(rect, rounding, stroke);
		}

		response
	}
}
