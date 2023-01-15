use egui::{Rect, Response, Sense, TextureHandle, TextureId, Ui, Vec2, Widget};

use super::image_size;

#[derive(Clone, Copy)]
pub struct Zoom {
	/// 0, 0 = center
	pub center: Vec2,
	/// 0 = no zoom
	pub zoom: f32,
}

impl Default for Zoom {
	fn default() -> Self {
		Self {
			center: Vec2 { x: 0.0, y: 0.0 },
			zoom: 0.0,
		}
	}
}

impl Zoom {
	fn apply(self, rect: Rect) -> Rect {
		rect.expand2(rect.size() * self.zoom)
	}

	pub fn update_from_input(&mut self, input: &egui::InputState) {
		self.zoom += input.scroll_delta.y * 0.01;
		self.clamp();
	}

	pub fn clamp(&mut self) {
		self.zoom = self.zoom.clamp(-0.45, 10.0);
	}
}

/// Similar to `egui::widgets::Image` but preserves the aspect ratio of the texture.
pub struct Image {
	texture: TextureId,
	actual_size: Vec2,
	zoom: Zoom,
	clickable: bool,
}

impl Image {
	pub fn new(texture: TextureId, size: Vec2) -> Self {
		Self {
			texture,
			actual_size: size,
			zoom: Zoom::default(),
			clickable: false,
		}
	}

	pub fn for_texture(texture: &TextureHandle) -> Self {
		Self::new(texture.id(), texture.size_vec2())
	}

	pub fn zoom(self, zoom: Zoom) -> Self {
		Self { zoom, ..self }
	}

	pub fn clickable(self, clickable: bool) -> Self {
		Self { clickable, ..self }
	}

	/// Returns the actual rect that the image filled
	pub fn paint_at(self, ui: &mut Ui, available_rect: Rect) -> Rect {
		// Create a child UI so we can set the clip of the painter
		let mut ui = ui.child_ui(available_rect, *ui.layout());
		ui.set_clip_rect(available_rect);

		let available_size = available_rect.size();
		let scaled_size = image_size(self.actual_size, available_size);
		let mut image_rect = ui
			.layout()
			.align_size_within_rect(scaled_size, available_rect);

		image_rect = self.zoom.apply(image_rect);

		egui::widgets::Image::new(self.texture, scaled_size).paint_at(&mut ui, image_rect);

		image_rect
	}

	fn sense(&self) -> Sense {
		if self.clickable {
			Sense::click()
		} else {
			Sense::hover()
		}
	}
}

impl Widget for Image {
	fn ui(self, ui: &mut Ui) -> Response {
		let (id, space) = ui.allocate_space(ui.available_size());
		let sense = self.sense();
		let image_rect = self.paint_at(ui, space);
		ui.interact(image_rect, id, sense)
	}
}
