use ::image::ImageFormat;
use eframe::CreationContext;
use egui::style::Margin;
use egui::{Color32, Context, Frame, Painter, Rect, Rounding, Vec2};

pub use self::image::init_timezone;
use self::next_path::Direction as NextPathDirection;
use self::state::play::State as PlayState;
use self::state::{NavigationMode, State as ImageState};
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
}

pub struct App {
	config: Config,
	image_state: ImageState,
	settings_open: bool,
	internal_open: bool,
	slideshow: SlideshowState,
}

impl App {
	#[allow(clippy::needless_pass_by_value)] // consistency
	pub fn new(Args { paths }: Args, config: Config, cc: &CreationContext<'_>) -> Self {
		let (first_path, navigation_mode) = if paths.len() >= 2 {
			(Some(paths[0].clone()), NavigationMode::specified(paths))
		} else {
			(paths.into_iter().next(), NavigationMode::InDirectory)
		};

		let cache_size = config.cache_size;

		let mut ret = Self {
			config,
			image_state: ImageState::new(cc.egui_ctx.clone(), cache_size, navigation_mode),
			settings_open: false,
			internal_open: false,
			slideshow: SlideshowState::default(),
		};

		if let Some(first_path) = first_path {
			ret.image_state.open(first_path);
		}

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
				let response =
					ui.add(egui::Label::new(current_path.display().to_string()).sense(egui::Sense::click()));
				let clicked = response.clicked();
				let show_copied =
					ctx.animate_bool_with_time(response.id, clicked, ctx.style().animation_time * 2.0) > 0.0;
				response.on_hover_text(if show_copied {
					"Copied!"
				} else {
					"Click to copy"
				});
				if clicked {
					ui.output().copied_text = current_path.display().to_string();
				}
			}
		};

		let mut right = |this: &mut Self, ui: &mut egui::Ui| {
			ui.toggle_value(&mut this.settings_open, "???")
				.on_hover_text("Toggle settings window");

			{
				let mut fullscreen = frame.info().window_info.fullscreen;
				if ui
					.toggle_value(&mut fullscreen, "???")
					.on_hover_text("Toggle fullscreen")
					.changed()
				{
					frame.set_fullscreen(fullscreen);
				}
			}

			this.config.light_dark_toggle_button(ui);

			if this.image_state.current.is_some() {
				if ui.button("????").clicked() {
					this.image_state.trash_current();
				}

				let mut slideshow_active = this.slideshow.is_active();
				let icon = if slideshow_active { "???" } else { "???" };
				let changed = ui.toggle_value(&mut slideshow_active, icon).changed();

				if changed {
					if slideshow_active {
						this.slideshow.start(&this.config);
					} else {
						this.slideshow.stop();
					}
				}
			}

			if this
				.image_state
				.current
				.as_ref()
				.map_or(false, |current| current.inner.is_ok())
			{
				ui.toggle_value(&mut this.config.show_sidebar, "???")
					.on_hover_text("Toggle sidebar");
			}

			if this.image_state.current.as_ref().map_or(false, |current| {
				current
					.inner
					.as_ref()
					.map_or(false, |inner| inner.image.is_animated())
			}) {
				ui.toggle_value(&mut this.config.show_frames, "????")
					.on_hover_text("Toggle frames");
			}

			if this.image_state.waiting() {
				ui.spinner().on_hover_text("Loading");
			}
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

		let Some(state::OpenImage { inner: Ok(state::OpenImageInner { image, .. }), .. }) = &self.image_state.current else { return };

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

	fn show_frames(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		if !self.config.show_frames {
			return;
		}

		let Some(state::OpenImage { inner: Ok(
				state::OpenImageInner {
				play_state: PlayState::Animated {
					current_frame,
					playing,
				},
				image,
				..
				}
			)
				, ..}) = &mut self.image_state.current else { return; };
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
		let elapsed = ctx.input().unstable_dt;

		let next_from_slideshow = self
			.slideshow
			.advance(&self.config, Duration::new_secs_f32_saturating(elapsed));

		if next_from_slideshow {
			if self.config.slideshow.shuffle {
				self.move_in(NextPathDirection::Random);
			} else {
				self.move_right();
			}
		}

		if let SlideshowState::Active { remaining } = self.slideshow {
			ctx.request_repaint_after(remaining.into());
		}
	}

	fn show_central(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let panel = {
			let margin = if let Some(state::OpenImage { inner: Ok(..), .. }) = &self.image_state.current {
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
					play_state, image, ..
				}),
				..
			}) => {
				ui.centered_and_justified(|ui| {
					self.config.background.draw(ui.painter(), ui.max_rect());
					match play_state {
						PlayState::Single => {
							ui.add(widgets::Image::for_texture(&image.frames[0].0));
						}
						PlayState::Animated {
							current_frame,
							playing,
						} => {
							let (current_texture, _) = &image.frames[current_frame.idx];
							if ui
								.add(widgets::Image::for_texture(current_texture).sense(egui::Sense::click()))
								.clicked()
							{
								*playing = !*playing;
							}
							if *playing {
								let elapsed = ctx.input().unstable_dt;
								current_frame.advance(
									Duration::new_secs_f32_saturating(elapsed),
									image.frames.len(),
									|idx| image.frames[idx].1,
								);
								ctx.request_repaint_after(current_frame.remaining.into());
							}
						}
					}
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

	fn show_settings(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let window = egui::Window::new("Settings")
			.open(&mut self.settings_open)
			.resizable(false)
			.collapsible(true);
		window.show(ctx, |ui| {
			self.config.ui(ui);
		});
	}

	fn show_internal(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		let window = egui::Window::new("Internal State")
			.open(&mut self.internal_open)
			.resizable(false)
			.collapsible(true)
			.default_width(600.0);
		window.show(ctx, |ui| {
			self.image_state.internal_ui(ui);
		});
	}

	fn handle_global_keys(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
		use egui::Key;

		let key = |key: Key| ctx.input_mut().consume_key(egui::Modifiers::NONE, key);

		if key(Key::ArrowRight) {
			self.move_right();
		}
		if key(Key::ArrowLeft) {
			self.move_left();
		}
		if ctx
			.input_mut()
			.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, Key::I)
		{
			self.internal_open = !self.internal_open;
		}
	}

	fn move_in(&mut self, direction: NextPathDirection) {
		use std::task::Poll;

		match self.image_state.next_path(direction) {
			Poll::Ready(path) => self.image_state.open(path),
			Poll::Pending => { /* will be handled by handle_actor_responses */ }
		}
	}

	fn move_right(&mut self) {
		self.move_in(NextPathDirection::RIGHT);
	}

	fn move_left(&mut self) {
		self.move_in(NextPathDirection::LEFT);
	}

	fn handle_actor_responses(&mut self) {
		self.image_state.handle_actor_responses();
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
		if !ctx.wants_keyboard_input() {
			self.handle_global_keys(ctx, frame);
		}

		self.update_slideshow(ctx);
		self.handle_actor_responses();
		self.image_state.show_errors(ctx);

		self.show_settings(ctx, frame);
		self.show_internal(ctx, frame);
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
