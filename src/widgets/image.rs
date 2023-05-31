use egui::{Rect, Response, Sense, TextureHandle, TextureId, Ui, Vec2, Widget};

use super::image_size;

#[derive(Clone, Copy, PartialEq)]
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
	fn zoom_factor(self) -> f32 {
		2f32.powf(self.zoom)
	}

	fn apply(self, rect: Rect) -> Rect {
		let center = rect.center() + self.center;
		let size = rect.size() * self.zoom_factor();
		Rect::from_center_size(center, size)
	}

	pub fn update_from_response(&mut self, response: &Response) {
		if response.middle_clicked() {
			*self = Self::default();
			return;
		}

		self.center += response.drag_delta();
		if let Some(pointer) = response.hover_pos() {
			let pointer = pointer - response.rect.center();
			let old_zoom = self.zoom_factor();
			self.zoom += response.ctx.input().scroll_delta.y * 0.01;
			let zoom_delta = self.zoom_factor() / old_zoom;
			self.center -= pointer;
			self.center *= zoom_delta;
			self.center += pointer;
		}
	}

	pub fn modified(self) -> bool {
		self != Self::default()
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
		ui.set_clip_rect(available_rect.intersect(ui.clip_rect()));

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
			Sense::click_and_drag()
		} else {
			Sense::drag()
		}
	}
}

impl Widget for Image {
	fn ui(self, ui: &mut Ui) -> Response {
		let (id, space) = ui.allocate_space(ui.available_size());
		let sense = self.sense();
		self.paint_at(ui, space);
		// passing `space` for the interaction rect rather than the rect returned by `paint_at` so that the image can be zoomed/paused without the cursor necessarily being inside the actual image.
		// this makes zoom behavior more friendly, as the user can continue zooming even if the image has become small enough that the cursor is now outside of it.
		ui.interact(space, id, sense)
	}
}
