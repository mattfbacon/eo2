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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::{CreationContext, NativeOptions};
use egui::style::Margin;
use egui::{Color32, Context, Frame, Painter, Rect, Rounding, Vec2};
use image::error::ImageError;
use image::ImageFormat;

use self::args::Args;
use self::config::Config;
use self::seconds::Seconds;
use self::widgets::ShowColumnsExt as _;

mod args;
mod config;
mod error;
mod logic;
mod read_image;
mod seconds;
mod widgets;

fn main() -> Result<(), ()> {
	match main_() {
		Ok(()) => Ok(()),
		Err(error) => {
			error::show(error.0);
			Err(())
		}
	}
}

fn main_() -> Result<(), error::Stringed> {
	let args = args::load();
	let config = config::load()?;

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

	Ok(())
}

struct OpenImage {
	status: Result<(), ImageError>,
	path: PathBuf,
}

#[derive(Default)]
struct ImageState {
	cache: HashMap<PathBuf, logic::Image>,
	current: Option<OpenImage>,
}

impl ImageState {
	fn current_path(&self) -> Option<&Path> {
		self.current.as_ref().map(|open| &*open.path)
	}

	fn current(&self) -> Option<Result<&logic::Image, &ImageError>> {
		self.current.as_ref().map(|open| {
			open
				.status
				.as_ref()
				.map(|()| self.cache.get(&open.path).unwrap())
		})
	}

	fn current_mut(&mut self) -> Option<Result<&mut logic::Image, &ImageError>> {
		self.current.as_ref().map(|open| {
			open
				.status
				.as_ref()
				.map(|()| self.cache.get_mut(&open.path).unwrap())
		})
	}

	fn open(&mut self, ctx: &Context, path: PathBuf) {
		if self.cache.contains_key(&path) {
			self.current = Some(OpenImage {
				path,
				status: Ok(()),
			});
		} else {
			let status = logic::Image::load(ctx, &path).map(|image| {
				self.cache.insert(path.clone(), image);
			});
			self.current = Some(OpenImage { status, path });
		}
	}
}

#[derive(Default, Clone, Copy, Debug)]
enum SlideshowState {
	Active {
		remaining: Seconds,
	},
	#[default]
	Inactive,
}

impl SlideshowState {
	fn is_active(self) -> bool {
		match self {
			Self::Active { .. } => true,
			Self::Inactive => false,
		}
	}

	fn start(&mut self, config: &Config) {
		*self = Self::Active {
			remaining: config.slideshow.interval,
		};
	}

	fn advance(&mut self, config: &Config, secs: Seconds) -> bool {
		match self {
			Self::Active { remaining } => {
				let has_elapsed = remaining.advance(secs);
				if has_elapsed {
					self.start(config);
				}
				has_elapsed
			}
			Self::Inactive => false,
		}
	}

	fn stop(&mut self) {
		*self = Self::Inactive;
	}
}

struct App {
	config: Config,
	image_state: ImageState,
	settings_open: bool,
	slideshow: SlideshowState,
}

impl App {
	#[allow(clippy::needless_pass_by_value)] // consistency
	fn new(args: Args, config: Config, cc: &CreationContext<'_>) -> Self {
		let mut ret = Self {
			config,
			image_state: ImageState::default(),
			settings_open: false,
			slideshow: SlideshowState::default(),
		};
		ret.image_state.open(&cc.egui_ctx, args.path);
		ret
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

		if self.checkered {
			draw_checker(painter, rect, primary_color, secondary_color);
		} else {
			draw_solid(painter, rect, primary_color);
		}
	}
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

		let left = |this: &mut Self, ui: &mut egui::Ui| {
			if let Some(current_path) = this.image_state.current_path() {
				ui.label(current_path.display().to_string());
			}
		};

		let mut right = |this: &mut Self, ui: &mut egui::Ui| {
			{
				let mut fullscreen = frame.info().window_info.fullscreen;
				if ui
					.toggle_value(&mut fullscreen, "⛶")
					.on_hover_text("Toggle fullscreen")
					.changed()
				{
					frame.set_fullscreen(fullscreen);
				}
			}

			if this
				.image_state
				.current()
				.map_or(false, |current| current.is_ok())
			{
				ui.toggle_value(&mut this.config.show_sidebar, "ℹ")
					.on_hover_text("Toggle sidebar");
			}

			if this.image_state.current().map_or(false, |current| {
				current.map_or(false, logic::Image::is_animated)
			}) {
				ui.toggle_value(&mut this.config.show_frames, "🎞")
					.on_hover_text("Toggle frames");
			}

			ui.toggle_value(&mut this.settings_open, "⛭")
				.on_hover_text("Toggle settings window");

			{
				let mut slideshow_active = this.slideshow.is_active();
				let icon = if slideshow_active { "⏸" } else { "▶" };
				let changed = ui.toggle_value(&mut slideshow_active, icon).changed();

				if changed {
					if slideshow_active {
						this.slideshow.start(&this.config);
					} else {
						this.slideshow.stop();
					}
				}
			}

			this.config.light_dark_toggle_button(ui);
		};

		panel.show(ctx, |ui| {
			ui.horizontal(|ui| {
				use egui::{Align, Layout};

				ui.with_layout(Layout::left_to_right(Align::Center), |ui| left(self, ui));
				ui.with_layout(Layout::right_to_left(Align::Center), |ui| right(self, ui));
			});
		});
	}

	fn show_sidebar(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !self.config.show_sidebar {
			return;
		}

		let Some(Ok(image)) = self.image_state.current() else { return };

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

		let Some(Ok(
			logic::Image {
				inner: logic::ImageInner::Animated {
					textures,
					current_frame,
					playing,
				},
				..
			}
				)) = self.image_state.current_mut() else { return; };

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
				egui::ScrollArea::horizontal().show_columns(
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
								// always stop playing if a user selects a frame
								*playing = false;
								current_frame.move_to(idx, *frame_time);
							}
							// inline of on_hover_text that lazily evaluates `format!`
							response.on_hover_ui(|ui| {
								ui.label(format!("Frame {}, {}", idx + 1, textures[idx].1));
							});
						}
					},
				);
			});
	}

	fn update_slideshow(&mut self, ctx: &Context) {
		let elapsed = ctx.input().unstable_dt;

		let next_from_slideshow = self
			.slideshow
			.advance(&self.config, Seconds::new_secs_f32_saturating(elapsed));

		if next_from_slideshow {
			self.move_right(ctx);
		}

		if let SlideshowState::Active { remaining } = self.slideshow {
			ctx.request_repaint_after(remaining.into());
		}
	}

	fn show_central(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let panel = {
			let margin = if let Some(Ok(..)) = self.image_state.current() {
				0.0
			} else {
				8.0
			};
			let frame = Frame::none()
				.fill(ctx.style().visuals.window_fill())
				.inner_margin(margin);
			egui::CentralPanel::default().frame(frame)
		};

		panel.show(ctx, |ui| match self.image_state.current_mut() {
			Some(Ok(image)) => {
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
								let elapsed = ctx.input().unstable_dt;
								current_frame.advance(Seconds::new_secs_f32_saturating(elapsed), textures);
								ctx.request_repaint_after(current_frame.remaining.into());
							}
						}
					}
				});
			}
			Some(Err(error)) => {
				ui.heading(format!("error: {error}"));
			}
			None => {
				ui.heading("no image open");
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

	fn handle_global_keys(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		use egui::Key;

		let key = |key: Key| ctx.input_mut().consume_key(egui::Modifiers::NONE, key);

		if key(Key::ArrowRight) {
			self.move_right(ctx);
		} else if key(Key::ArrowLeft) {
			self.move_left(ctx);
		}
	}
}

#[derive(Clone, Copy)]
enum MoveDirection {
	Left,
	Right,
}

impl MoveDirection {
	fn before<T: Ord + ?Sized>(self, left: &T, right: &T) -> bool {
		match self {
			Self::Right => left < right,
			Self::Left => left > right,
		}
	}

	fn after<T: Ord + ?Sized>(self, left: &T, right: &T) -> bool {
		match self {
			Self::Right => left > right,
			Self::Left => left < right,
		}
	}
}

impl App {
	fn move_in(&mut self, ctx: &Context, direction: MoveDirection) {
		let Some(current_path) = self.image_state.current_path() else { return; };

		let parent = current_path.parent().unwrap(/* path must have a parent because it must be a file, though it may be empty. */);
		let current_name = current_path.file_name().unwrap(/* ditto */).to_string_lossy();

		let mut next_name: Option<String> = None;
		let mut wrapped_name: Option<String> = None;

		let readable_parent = if parent.as_os_str().is_empty() {
			".".as_ref()
		} else {
			parent
		};
		for entry in readable_parent.read_dir().unwrap().map(Result::unwrap) {
			if entry.file_type().unwrap().is_dir() {
				continue;
			}

			let this_name = entry.file_name();

			if image::ImageFormat::from_path(&this_name).is_err() {
				continue;
			}

			let this_name = this_name.to_string_lossy().into_owned();

			if wrapped_name
				.as_ref()
				.map_or(true, |first_name| direction.before(&this_name, first_name))
			{
				wrapped_name = Some(this_name.clone());
			}

			if direction.after(this_name.as_str(), current_name.as_ref())
				&& next_name
					.as_ref()
					.map_or(true, |next_name| direction.before(&this_name, next_name))
			{
				next_name = Some(this_name);
			}
		}

		let next_name = next_name.or(wrapped_name);
		if let Some(next_name) = next_name {
			self.image_state.open(ctx, parent.join(next_name));
		}
	}

	fn move_right(&mut self, ctx: &Context) {
		self.move_in(ctx, MoveDirection::Right);
	}

	fn move_left(&mut self, ctx: &Context) {
		self.move_in(ctx, MoveDirection::Left);
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		if !ctx.wants_keyboard_input() {
			self.handle_global_keys(ctx, frame);
		}

		self.update_slideshow(ctx);

		self.show_settings(ctx, frame);
		self.show_actions(ctx, frame);
		self.show_sidebar(ctx, frame);
		self.show_frames(ctx, frame);
		self.show_central(ctx, frame);
	}

	// NB save is not called without the persistence feature, so on_exit is a better option
	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		if let Err(error) = self.config.save() {
			error::show(error.to_string());
		}
	}
}
