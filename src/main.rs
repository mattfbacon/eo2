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

use eframe::{CreationContext, NativeOptions};
use egui::style::Margin;
use egui::{Color32, Context, Frame, Painter, Rect, Rounding, ScrollArea, Vec2};
use image::error::ImageError;
use image::ImageFormat;

use self::args::Args;
use self::config::Config;

mod args;
mod config;
mod logic;
mod read_image;
mod widgets;

fn main() {
	let args = args::load();
	let config = config::load();

	let mut native_options = NativeOptions::default();
	if let Some(theme) = config.theme {
		native_options.follow_system_theme = false;
		native_options.default_theme = theme;
	}
	eframe::run_native(
		"Image Viewer",
		native_options,
		Box::new(move |cc| Box::new(App::new(args, config, cc))),
	);
}

struct App {
	config: Config,
	path: String,
	image: Result<logic::Image, ImageError>,
	settings_open: bool,
}

impl App {
	fn new(args: Args, config: Config, cc: &CreationContext<'_>) -> Self {
		let image = logic::Image::load(&cc.egui_ctx, &args.path);
		Self {
			config,
			path: args.path.display().to_string(),
			image,
			settings_open: false,
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

		let dark = match self.color {
			config::BackgroundColor::Default => painter.ctx().style().visuals.dark_mode,
			config::BackgroundColor::Light => false,
			config::BackgroundColor::Dark => true,
		};

		let (primary_color, secondary_color) = {
			const CONTRAST: u8 = 12;
			if dark {
				const BASE: u8 = 27;
				(
					Color32::from_gray(BASE),
					Color32::from_gray(BASE + CONTRAST),
				)
			} else {
				const BASE: u8 = 248;
				(
					Color32::from_gray(BASE),
					Color32::from_gray(BASE - CONTRAST),
				)
			}
		};

		if self.checker {
			draw_checker(painter, rect, primary_color, secondary_color);
		} else {
			draw_solid(painter, rect, primary_color);
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

			if self.image.is_ok() {
				ui.toggle_value(&mut self.config.show_sidebar, "ℹ");
			}

			if self.image.as_ref().map_or(false, logic::Image::is_animated) {
				ui.toggle_value(&mut self.config.show_frames, "🎞");
			}

			ui.toggle_value(&mut self.settings_open, "⛭");

			self.config.light_dark_toggle_button(ui);
		};

		panel.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), left);
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
			});
		});
	}

	fn show_sidebar(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !self.config.show_sidebar {
			return;
		}

		let Ok(image) = &self.image else { return };

		egui::SidePanel::right("properties").show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				ui.heading("Properties");
			});

			widgets::KeyValue::new("properties-kv").show(ui, |mut rows| {
				rows.row("Width", |ui| ui.label(image.width.to_string()));
				rows.row("Height", |ui| ui.label(image.height.to_string()));
				rows.row("Format", |ui| ui.label(format_to_string(image.format)));
				rows.row("Kind", |ui| ui.label(image.inner.kind()));
			});
		});
	}

	fn show_frames(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !self.config.show_frames {
			return;
		}

		let Ok(logic::Image {
				inner: logic::ImageInner::Animated {
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
								current_frame.move_to(idx, *frame_time);
							}
							// inline of on_hover_text that lazily evaluates `format!`
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
						logic::ImageInner::Single(texture) => {
							ui.add(widgets::Image::for_texture(texture));
						}
						logic::ImageInner::Animated {
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

	fn show_settings(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let window = egui::Window::new("Settings")
			.open(&mut self.settings_open)
			.resizable(false)
			.collapsible(true);
		window.show(ctx, |ui| {
			self.config.ui(ui);
		});
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		self.show_settings(ctx, frame);
		self.show_actions(ctx, frame);
		self.show_sidebar(ctx, frame);
		self.show_frames(ctx, frame);
		self.show_central(ctx, frame);
	}

	// NB save is not called without the persistence feature, so on_exit is a better option
	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		self.config.save();
	}
}
