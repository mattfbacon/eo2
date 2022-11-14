use std::io::{BufRead, Seek, SeekFrom};
use std::path::Path;

use egui::Color32;
use image::error::{DecodingError, ImageError, ImageFormatHint};
use image::io::Limits;
use image::{AnimationDecoder, Delay, DynamicImage, ImageDecoder, ImageFormat};

pub type Frame = Vec<Color32>;

pub struct Image {
	pub format: ImageFormat,
	pub width: u32,
	pub height: u32,
	pub frames: Vec<(Frame, Delay)>,
}

trait DecoderVisitor {
	type Return;

	fn visit<'a, D: ImageDecoder<'a>>(
		self,
		decoder: D,
		format: ImageFormat,
	) -> Result<Self::Return, ImageError>;
	fn visit_animated<'a, D: AnimationDecoder<'a>>(
		self,
		decoder: D,
		format: ImageFormat,
	) -> Result<Self::Return, ImageError>;
}

fn load_decoder<V: DecoderVisitor>(
	reader: impl BufRead + Seek,
	format: ImageFormat,
	visitor: V,
) -> Result<V::Return, ImageError> {
	macro_rules! visitors {
		(@arm @png $($decoder:ident)::*) => {{
			let decoder = image::codecs:: $($decoder)::* ::new(reader)?;
			if decoder.is_apng() {
				visitor.visit_animated(decoder.apng(), format)
			} else {
				visitor.visit(decoder, format)
			}
		}};
		(@arm @animated $($decoder:ident)::*) => {
			visitor.visit_animated(image::codecs:: $($decoder)::* ::new(reader)?, format)
		};
		(@arm $($decoder:ident)::*) => {
			visitor.visit(image::codecs:: $($decoder)::* ::new(reader)?, format)
		};
		($($format:ident => $(@$special:tt)? $($decoder:ident)::*),* $(,)?) => {
			match format {
				$(ImageFormat::$format => visitors!(@arm $(@$special)? $($decoder)::*),)*
				_ => Err(ImageError::Unsupported(
					ImageFormatHint::Exact(format).into(),
				)),
			}
		};
	}

	visitors! {
		// Avif => avif::AvifDecoder,
		Png => @png png::PngDecoder,
		Gif => @animated gif::GifDecoder,
		Jpeg => jpeg::JpegDecoder,
		WebP => @animated webp::WebPDecoder,
		Tiff => tiff::TiffDecoder,
		Tga => tga::TgaDecoder,
		Dds => dds::DdsDecoder,
		Bmp => bmp::BmpDecoder,
		Ico => ico::IcoDecoder,
		Hdr => hdr::HdrAdapter,
		OpenExr => openexr::OpenExrDecoder,
		Pnm => pnm::PnmDecoder,
		Farbfeld => farbfeld::FarbfeldDecoder,
	}
}

struct Visitor;

impl DecoderVisitor for Visitor {
	type Return = Image;

	fn visit<'a, D: ImageDecoder<'a>>(
		self,
		mut decoder: D,
		format: ImageFormat,
	) -> Result<Image, ImageError> {
		let mut limits = Limits::default();
		limits.max_image_width = Some(1_000_000);
		limits.max_image_height = Some(1_000_000);
		limits.max_alloc = Some(1024 * 1024 * 1024); // 1 GB
		limits.reserve(decoder.total_bytes())?;
		decoder.set_limits(limits)?;
		let image = DynamicImage::from_decoder(decoder)?.into_rgba8();
		let (width, height) = image.dimensions();
		Ok(Image {
			format,
			width,
			height,
			frames: vec![(
				bytemuck::allocation::cast_vec(image.into_raw()),
				Delay::from_numer_denom_ms(1, 1), // doesn't matter
			)],
		})
	}

	fn visit_animated<'a, D: AnimationDecoder<'a>>(
		self,
		decoder: D,
		format: ImageFormat,
	) -> Result<Image, ImageError> {
		let mut size = None;
		let frames = decoder
			.into_frames()
			.map(|frame| {
				frame.map(|frame| {
					let this_size = frame.buffer().dimensions();
					match size {
						None => size = Some(this_size),
						Some(old_size) => assert_eq!(old_size, this_size),
					}
					assert!(frame.top() == 0 && frame.left() == 0);
					let delay = frame.delay();
					(
						bytemuck::allocation::cast_vec(frame.into_buffer().into_raw()),
						delay,
					)
				})
			})
			.collect::<Result<Vec<_>, _>>()?;
		let (width, height) = size.ok_or_else(|| {
			ImageError::Decoding(DecodingError::new(
				ImageFormatHint::Exact(format),
				"no frames",
			))
		})?;
		Ok(Image {
			format,
			width,
			height,
			frames,
		})
	}
}

impl Image {
	pub fn read(path: &Path) -> Result<Self, ImageError> {
		let reader = image::io::Reader::open(path)?;
		let reader = reader.with_guessed_format()?;
		let format = reader.format().ok_or_else(|| {
			ImageError::Unsupported(ImageFormatHint::PathExtension(path.to_owned()).into())
		})?;
		let mut reader = reader.into_inner();
		reader.seek(SeekFrom::Start(0)).unwrap();
		load_decoder(reader, format, Visitor)
	}
}
