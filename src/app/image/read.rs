use std::io::{BufRead, Seek};
use std::path::Path;

use egui::Color32;
use image::error::{DecodingError, ImageError, ImageFormatHint, ImageResult};
use image::io::Limits;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat};

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
	macro_rules! visitors {
		(@arm @png $($decoder:ident)::*) => {{
			let decoder = image::codecs:: $($decoder)::* ::new(reader)?;
			if decoder.is_apng()? {
				visitor.visit_animated(decoder.apng()?, format)
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
		Avif => avif::AvifDecoder,
		Png => @png png::PngDecoder,
		Gif => @animated gif::GifDecoder,
		Jpeg => jpeg::JpegDecoder,
		WebP => @animated webp::WebPDecoder,
		Tiff => tiff::TiffDecoder,
		Tga => tga::TgaDecoder,
		Dds => dds::DdsDecoder,
		Bmp => bmp::BmpDecoder,
		Ico => ico::IcoDecoder,
		Hdr => hdr::HdrDecoder,
		OpenExr => openexr::OpenExrDecoder,
		Pnm => pnm::PnmDecoder,
		Qoi => qoi::QoiDecoder,
		Farbfeld => farbfeld::FarbfeldDecoder,
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
		let mut limits = Limits::default();
		limits.max_image_width = Some(1_000_000);
		limits.max_image_height = Some(1_000_000);
		limits.max_alloc = Some(1024 * 1024 * 1024); // 1 GB
		limits.reserve(decoder.total_bytes())?;
		decoder.set_limits(limits)?;
		let image = DynamicImage::from_decoder(decoder)?.into_rgba8();
		let (width, height) = image.dimensions();
		// `egui::Color32` and `image::Rgba<u8>` have the same size (4) and align (1) so `cast_vec` will never fail
		let frame = bytemuck::allocation::cast_vec(image.into_raw());
		Ok(Image {
			format,
			width,
			height,
			frames: vec![(
				(self.frame_mapper)(width, height, frame.into()),
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
				let frame = bytemuck::allocation::cast_vec(frame.into_buffer().into_raw());
				Ok((
					(self.frame_mapper)(width, height, frame.into()),
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
	let reader = image::io::Reader::open(path)?;
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
