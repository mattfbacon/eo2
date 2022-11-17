use std::path::Path;

use egui::{Color32, Context, TextureFilter, TextureHandle};
use image::{ImageFormat, ImageResult};

use crate::read_image;
use crate::seconds::Seconds;

#[derive(Debug, Clone, Copy)]
pub struct CurrentFrame {
	pub idx: usize,
	pub remaining: Seconds,
}

impl CurrentFrame {
	pub fn new(remaining: impl Into<Seconds>) -> Self {
		Self::new_at(0, remaining.into())
	}

	pub fn new_at(idx: usize, remaining: impl Into<Seconds>) -> Self {
		Self {
			idx,
			remaining: remaining.into(),
		}
	}

	pub fn move_to(&mut self, idx: usize, remaining: impl Into<Seconds>) {
		*self = Self::new_at(idx, remaining.into());
	}

	pub fn advance(&mut self, elapsed: Seconds, frames: &[(TextureHandle, Seconds)]) {
		// note: this intentionally never advances more than one frame
		if self.remaining.advance(elapsed) {
			self.idx = (self.idx + 1) % frames.len();
			self.remaining = frames[self.idx].1;
		}
	}
}

pub enum ImageInner {
	Animated {
		textures: Vec<(TextureHandle, Seconds)>,
		current_frame: CurrentFrame,
		playing: bool,
	},
	Single(TextureHandle),
}

impl ImageInner {
	pub fn kind(&self) -> &'static str {
		match self {
			Self::Animated { .. } => "Animated",
			Self::Single(..) => "Static",
		}
	}

	pub fn is_animated(&self) -> bool {
		matches!(self, Self::Animated { .. })
	}
}

pub struct Image {
	pub format: ImageFormat,
	pub width: u32,
	pub height: u32,
	pub inner: ImageInner,
}

impl Image {
	pub fn is_animated(&self) -> bool {
		self.inner.is_animated()
	}

	pub fn load(ctx: &Context, path: &Path) -> ImageResult<Self> {
		fn load_texture(
			ctx: &Context,
			width: u32,
			height: u32,
			frame: Vec<Color32>,
			idx: usize,
		) -> TextureHandle {
			ctx.load_texture(
				idx.to_string(),
				egui::ColorImage {
					size: [width.try_into().unwrap(), height.try_into().unwrap()],
					pixels: frame,
				},
				TextureFilter::Linear,
			)
		}

		let read_image::Image {
			format,
			width,
			height,
			frames,
		} = read_image::Image::read(path)?;
		let inner = match frames.len() {
			0 => unreachable!(),
			1 => ImageInner::Single(load_texture(
				ctx,
				width,
				height,
				frames.into_iter().next().unwrap().0.into(),
				0,
			)),
			_ => {
				let current_delay = frames[0].1;
				let textures = frames
					.into_iter()
					.enumerate()
					.map(|(idx, (frame, delay))| {
						let texture = load_texture(ctx, width, height, frame.into(), idx);
						(texture, delay)
					})
					.collect();
				ImageInner::Animated {
					textures,
					current_frame: CurrentFrame::new(current_delay),
					playing: true,
				}
			}
		};

		Ok(Image {
			format,
			width,
			height,
			inner,
		})
	}
}
