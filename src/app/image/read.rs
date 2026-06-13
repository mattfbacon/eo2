use std::io::{BufRead, Seek};
use std::path::Path;

use egui::Color32;
use image::error::{DecodingError, ImageError, ImageFormatHint, ImageResult};
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, Limits};

use super::{Image, Metadata};
use crate::duration::Duration;

type Frame = Box<[Color32]>;

trait DecoderVisitor {
	type Return;

	fn visit<D: ImageDecoder>(self, decoder: D, format: ImageFormat) -> ImageResult<Self::Return>;
	fn visit_animated<'a, D: AnimationDecoder<'a>>(
		self,
		decoder: D,
		format: ImageFormat,
	) -> ImageResult<Self::Return>;
}

fn load_decoder<V: DecoderVisitor>(
	reader: impl BufRead + Seek,
	format: ImageFormat,
	visitor: V,
) -> ImageResult<V::Return> {
	macro_rules! dec {
		($($name:ident)::+) => {
			visitor.visit(image::codecs::$($name)::+::new(reader)?, format)
		};
	}

	match format {
		ImageFormat::Png => {
			let decoder = image::codecs::png::PngDecoder::new(reader)?;
			if decoder.is_apng()? {
				visitor.visit_animated(decoder.apng()?, format)
			} else {
				visitor.visit(decoder, format)
			}
		}
		ImageFormat::Gif => {
			visitor.visit_animated(image::codecs::gif::GifDecoder::new(reader)?, format)
		}
		ImageFormat::WebP => {
			let decoder = image::codecs::webp::WebPDecoder::new(reader)?;
			if decoder.has_animation() {
				visitor.visit_animated(decoder, format)
			} else {
				visitor.visit(decoder, format)
			}
		}
		ImageFormat::Avif => dec!(avif::AvifDecoder),
		ImageFormat::Jpeg => dec!(jpeg::JpegDecoder),
		ImageFormat::Tiff => dec!(tiff::TiffDecoder),
		ImageFormat::Tga => dec!(tga::TgaDecoder),
		ImageFormat::Dds => dec!(dds::DdsDecoder),
		ImageFormat::Bmp => dec!(bmp::BmpDecoder),
		ImageFormat::Ico => dec!(ico::IcoDecoder),
		ImageFormat::Hdr => dec!(hdr::HdrDecoder),
		ImageFormat::OpenExr => dec!(openexr::OpenExrDecoder),
		ImageFormat::Pnm => dec!(pnm::PnmDecoder),
		ImageFormat::Qoi => dec!(qoi::QoiDecoder),
		ImageFormat::Farbfeld => dec!(farbfeld::FarbfeldDecoder),
		_ => Err(ImageError::Unsupported(
			ImageFormatHint::Exact(format).into(),
		)),
	}
}

struct Visitor<F> {
	frame_mapper: F,
	metadata: Metadata,
}

impl<OutFrameType, F: FnMut(u32, u32, Frame) -> OutFrameType> DecoderVisitor for Visitor<F> {
	type Return = Image<OutFrameType>;

	fn visit<D: ImageDecoder>(
		mut self,
		mut decoder: D,
		format: ImageFormat,
	) -> ImageResult<Self::Return> {
		decoder.set_limits({
			let mut limits = Limits::default();
			limits.max_image_width = Some(1_000_000);
			limits.max_image_height = Some(1_000_000);
			limits.max_alloc = Some(1024 * 1024 * 1024); // 1 GB
			limits.reserve(decoder.total_bytes())?;
			limits
		})?;

		let orientation = decoder.orientation()?;

		let mut image = DynamicImage::from_decoder(decoder)?;
		image.apply_orientation(orientation);

		let image = image.into_rgba8();

		let (width, height) = image.dimensions();

		let frame = image
			.pixels()
			.map(|&image::Rgba([r, g, b, a])| Color32::from_rgba_premultiplied(r, g, b, a))
			.collect::<Box<[_]>>();

		Ok(Image {
			format,
			width,
			height,
			frames: vec![(
				(self.frame_mapper)(width, height, frame),
				Duration::new_secs(1).unwrap(), // this value is ignored
			)],
			metadata: self.metadata,
		})
	}

	fn visit_animated<'a, D: AnimationDecoder<'a>>(
		mut self,
		decoder: D,
		format: ImageFormat,
	) -> ImageResult<Self::Return> {
		let error = |error| ImageError::Decoding(DecodingError::new(format.into(), error));
		let partial_frame_error = || error("partial frames are unimplemented");

		let mut size = None;
		let frames = decoder
			.into_frames()
			.map(|frame| {
				let frame = frame?;

				let this_size = frame.buffer().dimensions();
				match size {
					None => {
						size = Some(this_size);
					}
					Some(old_size) => {
						if old_size != this_size {
							return Err(partial_frame_error());
						}
					}
				}
				let (width, height) = this_size;

				if frame.top() != 0 || frame.left() != 0 {
					return Err(partial_frame_error());
				}

				let delay = frame.delay();
				let frame = frame
					.buffer()
					.pixels()
					.map(|&image::Rgba([r, g, b, a])| Color32::from_rgba_premultiplied(r, g, b, a))
					.collect::<Box<[_]>>();

				Ok((
					(self.frame_mapper)(width, height, frame),
					delay.try_into().map_err(|_| error("delay out of range"))?,
				))
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
			metadata: self.metadata,
		})
	}
}

pub fn read<OutFrameType>(
	path: &Path,
	load_frame: impl FnMut(u32, u32, Frame) -> OutFrameType,
) -> ImageResult<Image<OutFrameType>> {
	let metadata = Metadata::from_path(path)?;
	let reader = image::ImageReader::open(path)?;
	let reader = reader.with_guessed_format()?;
	let format = reader.format().ok_or_else(|| {
		ImageError::Unsupported(ImageFormatHint::PathExtension(path.to_owned()).into())
	})?;
	let mut reader = reader.into_inner();
	reader.rewind()?;
	load_decoder(
		reader,
		format,
		Visitor {
			frame_mapper: load_frame,
			metadata,
		},
	)
}
