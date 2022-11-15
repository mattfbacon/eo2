use egui::{Rect, Response, Sense, TextureHandle, TextureId, Ui, Vec2, Widget};

use super::image_size;

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
