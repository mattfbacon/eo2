use std::path::Path;

use egui::{Context, TextureFilter, TextureHandle, TextureOptions};
use image::{ImageFormat, ImageResult};
use once_cell::sync::Lazy;

use crate::duration::Duration;

mod read;

static TIMEZONE: Lazy<time::UtcOffset> =
	Lazy::new(|| time::UtcOffset::current_local_offset().unwrap());

pub fn init_timezone() {
	Lazy::force(&TIMEZONE);
}

#[derive(Debug)]
pub struct Metadata {
	pub file_size: u64,
	pub mtime: Option<String>,
}

impl Metadata {
	fn from_path(path: &Path) -> std::io::Result<Self> {
		let metadata = std::fs::metadata(path)?;
		Ok(Self {
			file_size: metadata.len(),
			mtime: metadata.modified().ok().map(|sys| {
				time::OffsetDateTime::from(sys)
					.to_offset(*TIMEZONE)
					.format(time::macros::format_description!(
						"[year]-[month]-[day] [hour]:[minute]:[second]"
					))
					.unwrap()
			}),
		})
	}
}

#[derive(Debug)]
pub struct Image<FrameType = TextureHandle> {
	pub format: ImageFormat,
	pub width: u32,
	pub height: u32,
	pub frames: Vec<(FrameType, Duration)>,
	pub metadata: Metadata,
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
				TextureOptions {
					magnification: TextureFilter::Nearest,
					minification: TextureFilter::Linear,
				},
			)
		})?;
		Ok(image)
	}

	pub fn size_in_memory(&self) -> usize {
		self
			.frames
			.iter()
			.map(|(frame, _delay)| {
				let [width, height] = frame.size();
				let pixel_size = std::mem::size_of::<egui::Color32>();
				width.saturating_mul(height).saturating_mul(pixel_size)
			})
			.sum()
	}
}
