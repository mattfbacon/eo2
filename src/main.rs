#![deny(
	absolute_paths_not_starting_with_crate,
	future_incompatible,
	keyword_idents,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	non_ascii_idents,
	nonstandard_style,
	noop_method_call,
	pointer_structural_match,
	private_in_public,
	rust_2018_idioms,
	unused_qualifications
)]
#![warn(clippy::pedantic)]
#![allow(clippy::let_underscore_drop)]
#![forbid(unsafe_code)]

use std::path::Path;

use eframe::{CreationContext, NativeOptions};
use egui::style::Margin;
use egui::{
	Color32, Context, Frame, Painter, Rect, Rounding, ScrollArea, TextureFilter, TextureHandle, Vec2,
};
use image::error::ImageError;
use image::ImageFormat;

use self::args::Args;
use self::read_image::Seconds;

mod args;
mod config;
mod read_image;
mod widgets;

fn main() {
	let args = args::load();

	let native_options = NativeOptions::default();
	eframe::run_native(
		"Image Viewer",
		native_options,
		Box::new(move |cc| Box::new(App::new(&args, cc))),
	);
}

#[derive(Debug, Clone, Copy)]
struct CurrentFrame {
	idx: usize,
	remaining: Seconds,
}

impl CurrentFrame {
	fn new(remaining: impl Into<Seconds>) -> Self {
		Self {
			idx: 0,
			remaining: remaining.into(),
		}
	}

	fn advance(&mut self, elapsed: f32, frames: &[(TextureHandle, Seconds)]) {
		// note: this intentionally never advances more than one frame
		if self.remaining.advance(elapsed) {
			self.idx = (self.idx + 1) % frames.len();
			self.remaining = frames[self.idx].1;
		}
	}
}

enum ImageInner {
	Animated {
		textures: Vec<(TextureHandle, Seconds)>,
		current_frame: CurrentFrame,
		playing: bool,
	},
	Single(TextureHandle),
}

impl ImageInner {
	fn kind(&self) -> &'static str {
		match self {
			Self::Animated { .. } => "Animated",
			Self::Single(..) => "Static",
		}
	}
}

struct Image {
	format: ImageFormat,
	width: u32,
	height: u32,
	inner: ImageInner,
}

fn load_image(ctx: &Context, path: &Path) -> Result<Image, ImageError> {
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

struct App {
	config: config::Config,
	path: String,
	image: Result<Image, ImageError>,
}

impl App {
	fn new(args: &Args, cc: &CreationContext<'_>) -> Self {
		let config = config::load();
		let image = load_image(&cc.egui_ctx, &args.path);
		Self {
			config,
			path: args.path.display().to_string(),
			image,
		}
	}
}

fn format_to_string(format: ImageFormat) -> &'static str {
	match format {
		ImageFormat::Png => "PNG",
		ImageFormat::Jpeg => "JPEG",
		ImageFormat::Gif => "GIF",
		ImageFormat::WebP => "WEBP",
		ImageFormat::Pnm => "PNM",
		ImageFormat::Tiff => "TIFF",
		ImageFormat::Tga => "TGA",
		ImageFormat::Dds => "DDS",
		ImageFormat::Bmp => "BMP",
		ImageFormat::Ico => "ICO",
		ImageFormat::Hdr => "HDR",
		ImageFormat::OpenExr => "OpenEXR",
		ImageFormat::Farbfeld => "Farbfeld",
		ImageFormat::Avif => "AVIF",
		_ => "unknown",
	}
}

impl config::Background {
	fn draw(self, painter: &Painter, rect: Rect) {
		fn draw_solid(painter: &Painter, rect: Rect, color: Color32) {
			painter.rect_filled(rect, Rounding::none(), color);
		}

		fn draw_checker(painter: &Painter, rect: Rect, color1: Color32, color2: Color32) {
			const CHECKER_SIZE: u32 = 20;
			#[allow(clippy::cast_precision_loss)]
			const CHECKER_VEC: Vec2 = Vec2::splat(CHECKER_SIZE as f32);
			const STEP: usize = (CHECKER_SIZE * 2) as usize;

			painter.rect_filled(rect, Rounding::none(), color1);

			let base_pos = rect.left_top();
			// only add rects for color2
			let painter = painter.with_clip_rect(rect);
			// draw two rows at a time; one offset by CHECKER_SIZE
			for y in (0..az::cast(rect.height())).step_by(STEP) {
				for x in (0..az::cast(rect.width())).step_by(STEP) {
					painter.rect_filled(
						Rect::from_min_size(base_pos + Vec2::new(az::cast(x), az::cast(y)), CHECKER_VEC),
						Rounding::none(),
						color2,
					);
					painter.rect_filled(
						Rect::from_min_size(
							base_pos + Vec2::new(az::cast(x), az::cast(y)) + CHECKER_VEC,
							CHECKER_VEC,
						),
						Rounding::none(),
						color2,
					);
				}
			}
		}

		match self {
			Self::Default => draw_solid(
				painter,
				rect,
				painter.ctx().style().visuals.widgets.noninteractive.bg_fill,
			),
			Self::Light => draw_solid(painter, rect, Color32::from_gray(250)),
			Self::Dark => draw_solid(painter, rect, Color32::from_gray(10)),
			Self::LightChecker => {
				draw_checker(
					painter,
					rect,
					Color32::from_gray(250),
					Color32::from_gray(230),
				);
			}
			Self::DarkChecker => {
				draw_checker(
					painter,
					rect,
					Color32::from_gray(10),
					Color32::from_gray(30),
				);
			}
		}
	}
}

// based on the `show_rows` implementation in egui.
fn show_columns(
	scroll_area: ScrollArea,
	ui: &mut egui::Ui,
	item_width_without_spacing: f32,
	total_items: usize,
	add_contents: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>),
) {
	use egui::NumExt as _;

	let spacing = ui.spacing().item_spacing;
	let item_width_with_spacing = item_width_without_spacing + spacing.x;
	scroll_area.show_viewport(ui, |ui, viewport| {
		ui.set_width({
			let total_items_f: f32 = az::cast(total_items);
			let including_last_padding = item_width_with_spacing * total_items_f;
			let width = including_last_padding - spacing.x;
			width.at_least(0.0)
		});

		let min_col = az::cast::<_, usize>((viewport.min.x / item_width_with_spacing).floor());
		let max_col = az::cast::<_, usize>((viewport.max.x / item_width_with_spacing).ceil()) + 1;
		let max_col = max_col.at_most(total_items);

		let x_min = ui.max_rect().left() + az::cast::<_, f32>(min_col) * item_width_with_spacing;
		let x_max = ui.max_rect().left() + az::cast::<_, f32>(max_col) * item_width_with_spacing;

		let rect = Rect::from_x_y_ranges(x_min..=x_max, ui.max_rect().y_range());

		ui.allocate_ui_at_rect(rect, |ui| {
			ui.skip_ahead_auto_ids(min_col);
			ui.horizontal(|ui| {
				add_contents(ui, min_col..max_col);
			});
		});
	});
}

impl App {
	fn show_actions(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		let panel = {
			let style = ctx.style();
			let frame = Frame {
				inner_margin: Margin::symmetric(4.0, 2.0),
				rounding: Rounding::none(),
				fill: style.visuals.window_fill(),
				stroke: style.visuals.window_stroke(),
				..Default::default()
			};
			egui::TopBottomPanel::top("actions").frame(frame)
		};

		let left = |ui: &mut egui::Ui| {
			ui.label(&self.path);
		};

		let right = |ui: &mut egui::Ui| {
			let mut fullscreen = frame.info().window_info.fullscreen;
			if ui.toggle_value(&mut fullscreen, "⛶").changed() {
				frame.set_fullscreen(fullscreen);
			}

			if matches!(self.image, Ok(..)) {
				ui.toggle_value(&mut self.config.show_sidebar, "ℹ");
			}

			if matches!(
				self.image,
				Ok(Image {
					inner: ImageInner::Animated { .. },
					..
				})
			) {
				ui.toggle_value(&mut self.config.show_frames, "🎞");
			}
		};

		panel.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), left);
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
			});
		});
	}

	fn show_sidebar(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		fn key(ui: &mut egui::Ui, s: &str) {
			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				ui.label(s);
			});
		}

		if !self.config.show_sidebar {
			return;
		}

		let Ok(image) = &self.image else { return };

		let properties = |ui: &mut egui::Ui| {
			key(ui, "Width");
			ui.label(image.width.to_string());
			ui.end_row();

			key(ui, "Height");
			ui.label(image.height.to_string());
			ui.end_row();

			key(ui, "Format");
			ui.label(format_to_string(image.format));
			ui.end_row();

			key(ui, "Kind");
			ui.label(image.inner.kind());
			ui.end_row();
		};

		egui::SidePanel::right("properties").show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				ui.heading("Properties");
				egui::Grid::new("properties-grid")
					.num_columns(2)
					.show(ui, properties);
			});
		});
	}

	fn show_frames(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !self.config.show_frames {
			return;
		}

		let Ok(Image {
				inner: ImageInner::Animated {
					textures,
					current_frame,
					..
				},
				..
			}) = &mut self.image else { return; };

		let outer_frame_size = Vec2::splat(100.0); // XXX 100 is arbitrary; make it configurable?

		let frame_style = {
			let style = ctx.style();
			Frame {
				inner_margin: style.spacing.window_margin,
				fill: style.visuals.window_fill(),
				stroke: style.visuals.window_stroke(),
				..Frame::default()
			}
		};
		egui::TopBottomPanel::bottom("frames")
			.resizable(false)
			.frame(frame_style)
			.default_height(outer_frame_size.y + frame_style.inner_margin.sum().y) // may not include the scroll bar, but that's fine. this is just a decent baseline
			.show(ctx, |ui| {
				show_columns(
					egui::ScrollArea::horizontal(),
					ui,
					outer_frame_size.x,
					textures.len(),
					|ui, visible_range| {
						// iterate over an enumerated subslice with correct indices
						// XXX more elegant way to do that?
						for (idx, (texture, frame_time)) in textures[visible_range.clone()]
							.iter()
							.enumerate()
							.map(|(idx, v)| (idx + visible_range.start, v))
						{
							let button = widgets::ImageButton::new(texture, outer_frame_size)
								.selected(idx == current_frame.idx);
							let response = ui.add(button);
							if response.clicked() {
								*current_frame = CurrentFrame {
									idx,
									remaining: *frame_time,
								};
							}
							response.on_hover_ui(|ui| {
								ui.label(format!("Frame {}", idx + 1));
							});
							// TODO show frame times
						}
					},
				);
			});
	}

	fn show_central(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let panel = egui::CentralPanel::default().frame(Frame::none());
		panel.show(ctx, |ui| match &mut self.image {
			Ok(image) => {
				ui.centered_and_justified(|ui| {
					self.config.background.draw(ui.painter(), ui.max_rect());
					match &mut image.inner {
						ImageInner::Single(texture) => {
							ui.add(widgets::Image::for_texture(texture));
						}
						ImageInner::Animated {
							textures,
							current_frame,
							playing,
						} => {
							let (current_texture, _) = &textures[current_frame.idx];
							if ui
								.add(widgets::Image::for_texture(current_texture).sense(egui::Sense::click()))
								.clicked()
							{
								*playing = !*playing;
							}
							if *playing {
								let elapsed = ui.input().unstable_dt;
								current_frame.advance(elapsed, textures);
								ctx.request_repaint_after(current_frame.remaining.into());
							}
						}
					}
				});
			}
			Err(error) => {
				ui.heading(format!("error: {error:?}"));
			}
		});
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		self.show_actions(ctx, frame);
		self.show_sidebar(ctx, frame);
		self.show_frames(ctx, frame);
		self.show_central(ctx, frame);
	}
}
