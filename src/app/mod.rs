use std::path::Path;
use std::sync::Arc;

use ::image::ImageFormat;
use eframe::CreationContext;
use egui::style::Margin;
use egui::{Color32, Context, Frame, Modifiers, Painter, Rect, Rounding, Vec2, ViewportCommand};

pub use self::image::init_timezone;
use self::state::actor::{NavigationMode, NextPath, NextPathMode};
use self::state::play::State as PlayState;
use self::state::State as ImageState;
use crate::app::next_path::Direction;
use crate::args::Args;
use crate::config::Config;
use crate::duration::Duration;
use crate::widgets::ShowColumnsExt as _;
use crate::{config, error, widgets};

mod image;
mod next_path;
mod state;

#[derive(Default, Clone, Copy, Debug)]
enum SlideshowState {
	Active {
		remaining: Duration,
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

	fn advance(&mut self, config: &Config, secs: Duration) -> bool {
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

	fn set_active(&mut self, active: bool, config: &Config) {
		if active {
			self.start(config);
		} else {
			self.stop();
		}
	}

	fn toggle(&mut self, config: &Config) {
		self.set_active(!self.is_active(), config);
	}

	fn show_toggle(&mut self, ui: &mut egui::Ui, config: &Config) {
		let mut slideshow_active = self.is_active();
		let icon = if slideshow_active { "⏸" } else { "▶" };
		let changed = ui
			.toggle_value(&mut slideshow_active, icon)
			.on_hover_text("Toggle slideshow (s)")
			.changed();

		if changed {
			self.set_active(slideshow_active, config);
		}
	}
}

pub struct App {
	config: Config,
	image_state: ImageState,
	fullscreen: bool,
	settings_open: bool,
	internal_open: bool,
	asking_to_delete: Option<Arc<Path>>,
	slideshow: SlideshowState,
}

impl App {
	#[allow(clippy::needless_pass_by_value)] // consistency
	pub fn new(Args { paths }: Args, config: Config, cc: &CreationContext<'_>) -> Self {
		let navigation_mode = match paths.len() {
			0 => NavigationMode::Empty,
			1 => NavigationMode::InDirectory {
				current: paths.into_iter().next().unwrap(),
			},
			_ => NavigationMode::specified(paths),
		};

		let cache_size = config.cache_size;

		Self {
			config,
			image_state: ImageState::new(cc.egui_ctx.clone(), cache_size, navigation_mode),
			fullscreen: false,
			settings_open: false,
			internal_open: false,
			asking_to_delete: None,
			slideshow: SlideshowState::default(),
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
			painter.rect_filled(rect, Rounding::ZERO, color);
		}

		fn draw_checker(painter: &Painter, rect: Rect, color1: Color32, color2: Color32) {
			const CHECKER_SIZE: u32 = 20;
			#[allow(clippy::cast_precision_loss)]
			const CHECKER_VEC: Vec2 = Vec2::splat(CHECKER_SIZE as f32);
			const STEP: usize = (CHECKER_SIZE * 2) as usize;

			painter.rect_filled(rect, Rounding::ZERO, color1);

			let base_pos = rect.left_top();
			// only add rects for color2
			let painter = painter.with_clip_rect(rect);
			// draw two rows at a time; one offset by CHECKER_SIZE
			for y in (0..az::cast(rect.height())).step_by(STEP) {
				for x in (0..az::cast(rect.width())).step_by(STEP) {
					painter.rect_filled(
						Rect::from_min_size(base_pos + Vec2::new(az::cast(x), az::cast(y)), CHECKER_VEC),
						Rounding::ZERO,
						color2,
					);
					painter.rect_filled(
						Rect::from_min_size(
							base_pos + Vec2::new(az::cast(x), az::cast(y)) + CHECKER_VEC,
							CHECKER_VEC,
						),
						Rounding::ZERO,
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

fn show_fullscreen_toggle(ui: &mut egui::Ui) {
	let Some(mut fullscreen) = ui.input(|input| input.viewport().fullscreen) else {
		return;
	};
	if ui
		.toggle_value(&mut fullscreen, "⛶")
		.on_hover_text("Toggle fullscreen (f)")
		.changed()
	{
		let cmd = ViewportCommand::Fullscreen(!fullscreen);
		ui.ctx().send_viewport_cmd(cmd);
	}
}

#[derive(Debug, Clone, Copy)]
enum MoveMode {
	IgnoreSlideshow,
	RespectSlideshow,
}

impl App {
	fn move_in(&mut self, direction: Direction, mode: MoveMode) {
		let respect_slideshow = match mode {
			MoveMode::IgnoreSlideshow => false,
			MoveMode::RespectSlideshow => true,
		};
		let mode = if respect_slideshow && self.slideshow.is_active() && self.config.slideshow.shuffle {
			NextPathMode::Random
		} else {
			NextPathMode::Simple
		};
		let direction = NextPath { direction, mode };
		self.image_state.next_path(direction);
	}
}

impl App {
	fn show_actions_left(&mut self, ui: &mut egui::Ui) {
		if let Some(current_path) = self.image_state.current_path() {
			let response =
				ui.add(egui::Label::new(current_path.display().to_string()).sense(egui::Sense::click()));
			let clicked = response.clicked();
			let show_copied = ui.ctx().animate_bool_with_time(
				response.id,
				clicked,
				ui.ctx().style().animation_time * 2.0,
			) > 0.0;
			response.on_hover_text(if show_copied {
				"Copied!"
			} else {
				"Click to copy"
			});
			if clicked {
				let copied_text = current_path.display().to_string();
				ui.output_mut(|output| output.copied_text = copied_text);
			}
		}
	}

	fn show_actions_right(&mut self, ui: &mut egui::Ui) {
		let mut to_delete = None;

		ui.toggle_value(&mut self.settings_open, "⛭")
			.on_hover_text("Toggle settings window");

		show_fullscreen_toggle(ui);

		self.config.light_dark_toggle_button(ui);

		if let Some(current) = &mut self.image_state.current {
			let delete_button = ui.button("🗑");
			to_delete = delete_button.clicked().then(|| current.path.clone());
			delete_button.on_hover_text("Delete File");

			self.slideshow.show_toggle(ui, &self.config);

			if let Ok(inner) = &mut current.inner {
				if ui
					.add_enabled(inner.zoom.modified(), egui::Button::new("="))
					.on_hover_text("Reset zoom")
					.clicked()
				{
					inner.zoom = crate::widgets::image::Zoom::default();
				}

				ui.toggle_value(&mut self.config.show_sidebar, "ℹ")
					.on_hover_text("Toggle sidebar");

				if inner.image.is_animated() {
					ui.toggle_value(&mut self.config.show_frames, "🎞")
						.on_hover_text("Toggle frames");
				}
			}
		}

		if self.image_state.waiting() {
			ui.spinner().on_hover_text("Loading");
		}

		if let SlideshowState::Active { remaining } = self.slideshow {
			ui.label(format!("\u{2398} {} s", remaining.ceil_secs()));
			ui.ctx()
				.request_repaint_after(std::time::Duration::from_secs(1));
		}

		if let Some(to_delete) = to_delete {
			self.delete_file(ui, to_delete);
		}
	}

	fn delete_file(&mut self, ui: &egui::Ui, path: Arc<Path>) {
		if ui.input(|input| input.modifiers.shift) {
			self.asking_to_delete = None;
			self.image_state.delete_file(path);
		} else {
			self.asking_to_delete = Some(path);
		}
	}

	fn show_actions(&mut self, ctx: &Context) {
		let panel = {
			let style = ctx.style();
			let frame = Frame {
				inner_margin: Margin::symmetric(4.0, 2.0),
				rounding: Rounding::ZERO,
				fill: style.visuals.window_fill(),
				stroke: style.visuals.window_stroke(),
				..Default::default()
			};
			egui::TopBottomPanel::top("actions").frame(frame)
		};

		panel.show(ctx, |ui| {
			ui.horizontal(|ui| {
				use egui::{Align, Layout};

				ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
					self.show_actions_left(ui);
				});
				ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
					self.show_actions_right(ui);
				});
			});
		});
	}

	fn show_sidebar(&mut self, ctx: &Context) {
		if !self.config.show_sidebar {
			return;
		}

		let Some(state::OpenImage {
			inner: Ok(state::OpenImageInner { image, .. }),
			..
		}) = &self.image_state.current
		else {
			return;
		};

		egui::SidePanel::right("properties").show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				ui.heading("Properties");
			});

			widgets::KeyValue::new("properties-kv").show(ui, |mut rows| {
				rows.row("Width", |ui| ui.label(image.width.to_string()));
				rows.row("Height", |ui| ui.label(image.height.to_string()));
				rows.row("Format", |ui| ui.label(format_to_string(image.format)));
				rows.row("Kind", |ui| ui.label(image.kind().repr()));

				rows.separator();
				rows.row("File Size", |ui| {
					ui.label(humansize::format_size(
						image.metadata.file_size,
						humansize::DECIMAL,
					))
				});
				if let Some(mtime) = &image.metadata.mtime {
					rows.row("Modified", |ui| ui.label(mtime));
				}
			});
		});
	}

	fn show_frames(&mut self, ctx: &Context) {
		if !self.config.show_frames {
			return;
		}

		let Some(state::OpenImage {
			inner:
				Ok(state::OpenImageInner {
					play_state: PlayState::Animated {
						current_frame,
						playing,
					},
					image,
					..
				}),
			..
		}) = &mut self.image_state.current
		else {
			return;
		};
		let frames = &image.frames;

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
					frames.len(),
					|ui, visible_range| {
						// iterate over an enumerated subslice with correct indices
						// XXX more elegant way to do that?
						for (idx, (texture, frame_time)) in frames[visible_range.clone()]
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
								ui.label(format!("Frame {}, {}", idx + 1, frames[idx].1));
							});
						}
					},
				);
			});
	}

	fn update_slideshow(&mut self, ctx: &Context) {
		let elapsed = ctx.input(|input| input.unstable_dt);

		let next_from_slideshow = self
			.slideshow
			.advance(&self.config, Duration::new_secs_f32_saturating(elapsed));

		if next_from_slideshow {
			self.move_in(Direction::Right, MoveMode::RespectSlideshow);
		}

		if let SlideshowState::Active { remaining } = self.slideshow {
			ctx.request_repaint_after(remaining.into());
		}
	}

	fn show_central(&mut self, ctx: &Context) {
		let panel = {
			let margin = if matches!(
				self.image_state.current,
				Some(state::OpenImage { inner: Ok(..), .. })
			) {
				0.0
			} else {
				8.0
			};
			let frame = Frame::none()
				.fill(ctx.style().visuals.window_fill())
				.inner_margin(margin);
			egui::CentralPanel::default().frame(frame)
		};

		panel.show(ctx, |ui| match &mut self.image_state.current {
			Some(state::OpenImage {
				inner: Ok(state::OpenImageInner {
					play_state,
					image,
					zoom,
					..
				}),
				..
			}) => {
				ui.centered_and_justified(|ui| {
					self.config.background.draw(ui.painter(), ui.max_rect());
					let response = match play_state {
						PlayState::Single => {
							ui.add(widgets::Image::for_texture(&image.frames[0].0).zoom(*zoom))
						}
						PlayState::Animated {
							current_frame,
							playing,
						} => {
							let (current_texture, _) = &image.frames[current_frame.idx];
							let response = ui.add(
								widgets::Image::for_texture(current_texture)
									.clickable(true)
									.zoom(*zoom),
							);
							if response.clicked() {
								*playing = !*playing;
							}
							if *playing {
								let elapsed = ctx.input(|input| input.unstable_dt);
								current_frame.advance(
									Duration::new_secs_f32_saturating(elapsed),
									image.frames.len(),
									|idx| image.frames[idx].1,
								);
								ctx.request_repaint_after(current_frame.remaining.into());
							}
							response
						}
					};

					zoom.update_from_response(&response);
				});
			}
			Some(state::OpenImage {
				inner: Err(error), ..
			}) => {
				ui.heading(format!("error: {error}"));
			}
			None => {
				ui.heading("no image open");
			}
		});
	}

	fn show_settings(&mut self, ctx: &Context) {
		let window = egui::Window::new("Settings")
			.open(&mut self.settings_open)
			.resizable(false)
			.collapsible(true);
		window.show(ctx, |ui| {
			self.config.ui(ui);
		});
	}

	fn show_asking_to_delete(&mut self, ctx: &Context) {
		if self.asking_to_delete.is_none() {
			return;
		}

		let mut open = true;
		let window = egui::Window::new("Delete File?")
			.open(&mut open)
			.resizable(false)
			.collapsible(true);
		window.show(ctx, |ui| {
			ui.label(format!(
				"Delete {:?}?",
				self.asking_to_delete.as_ref().unwrap()
			));
			ui.allocate_ui_with_layout(
				Vec2::new(ui.max_rect().width(), 0.0),
				egui::Layout::right_to_left(egui::Align::BOTTOM),
				|ui| {
					if ui.button("Cancel").clicked() {
						self.asking_to_delete = None;
					}
					if ui.button("Delete").clicked() {
						let to_delete = self.asking_to_delete.take().unwrap();
						self.image_state.delete_file(to_delete);
					}
				},
			);
		});
		if !open {
			self.asking_to_delete = None;
		}
	}

	fn handle_global_keys(&mut self, ctx: &Context) {
		use egui::Key;

		const KEYS: &[(Key, Modifiers, Direction)] = &[
			(Key::ArrowLeft, Modifiers::NONE, Direction::Left),
			(Key::ArrowRight, Modifiers::NONE, Direction::Right),
			(Key::P, Modifiers::NONE, Direction::Left),
			(Key::N, Modifiers::NONE, Direction::Right),
			(Key::N, Modifiers::SHIFT, Direction::Left),
		];

		for &(key, modifiers, direction) in KEYS {
			debug_assert!(!modifiers.contains(Modifiers::ALT));
			let mode = ctx.input_mut(|input| {
				Some(if input.consume_key(modifiers, key) {
					MoveMode::RespectSlideshow
				} else if input.consume_key(modifiers | Modifiers::ALT, key) {
					MoveMode::IgnoreSlideshow
				} else {
					return None;
				})
			});
			if let Some(mode) = mode {
				self.move_in(direction, mode);
			}
		}

		if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::I)) {
			self.internal_open = !self.internal_open;
		}

		let key = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));

		if key(Key::S) {
			self.slideshow.toggle(&self.config);
		}

		if key(Key::F) {
			ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!self.fullscreen));
		}

		if key(Key::I) {
			self.config.show_sidebar ^= true;
		}

		if key(Key::C) {
			self.settings_open ^= true;
		}

		if key(Key::Q) {
			ctx.send_viewport_cmd(ViewportCommand::Close);
		}
	}

	fn handle_actor_responses(&mut self) {
		self.image_state.handle_actor_responses();
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !ctx.wants_keyboard_input() {
			self.handle_global_keys(ctx);
		}

		self.update_slideshow(ctx);
		self.handle_actor_responses();
		self.image_state.show_errors(ctx);

		self.show_settings(ctx);
		self.show_asking_to_delete(ctx);

		self.show_actions(ctx);
		self.show_sidebar(ctx);
		self.show_frames(ctx);
		self.show_central(ctx);
	}

	// NB save is not called without the persistence feature, so on_exit is a better option
	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		if let Err(error) = self.config.save() {
			error::show(error.to_string());
		}
	}
}
