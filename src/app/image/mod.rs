use std::path::Path;

use egui::{Context, TextureFilter, TextureHandle};
use image::{ImageFormat, ImageResult};

use crate::seconds::Seconds;

mod read;

pub struct Image<FrameType = TextureHandle> {
	pub format: ImageFormat,
	pub width: u32,
	pub height: u32,
	pub frames: Vec<(FrameType, Seconds)>,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
	Animated,
	Static,
}

impl Kind {
	pub fn repr(self) -> &'static str {
		match self {
			Self::Animated => "Animated",
			Self::Static => "Static",
		}
	}
}

impl Image {
	pub fn is_animated(&self) -> bool {
		self.frames.len() > 1
	}

	pub fn kind(&self) -> Kind {
		if self.is_animated() {
			Kind::Animated
		} else {
			Kind::Static
		}
	}

	pub fn load(ctx: &Context, path: &Path) -> ImageResult<Self> {
		let image = read::read(path, |width, height, frame| {
			ctx.load_texture(
				"", // has no importance
				egui::ColorImage {
					size: [width.try_into().unwrap(), height.try_into().unwrap()],
					pixels: frame.into(),
				},
				TextureFilter::Linear,
			)
		})?;
		Ok(image)
	}
}
