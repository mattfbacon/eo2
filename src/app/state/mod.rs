use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use egui::Context;
use image::error::ImageResult;

use self::actor::{LoadedImage, NavigationMode, NextPath, Response};
use super::image::Image;

pub mod actor;
pub mod play;

pub struct OpenImageInner {
	pub play_state: play::State,
	pub image: Arc<Image>,
	pub zoom: crate::widgets::image::Zoom,
}

pub struct OpenImage {
	pub inner: ImageResult<OpenImageInner>,
	pub path: Arc<Path>,
}

static ERRORS_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct State {
	pub current: Option<OpenImage>,
	actor: actor::Handle,
	errors: Vec<(egui::Id, String)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ErrorAcknowledged;

impl State {
	pub fn new(egui_ctx: Context, cache_size: NonZeroUsize, navigation_mode: NavigationMode) -> Self {
		Self {
			current: None,
			actor: actor::Handle::spawn(egui_ctx, navigation_mode, cache_size),
			errors: Vec::new(),
		}
	}

	pub fn waiting(&self) -> bool {
		self.actor.waiting()
	}

	fn push_error(&mut self, error: String) {
		let id =
			egui::Id::new("image-state-error").with(ERRORS_ID_COUNTER.fetch_add(1, Ordering::Relaxed));
		self.errors.push((id, error));
	}

	fn show_errors_inner(
		&mut self,
		mut show: impl FnMut(egui::Id, &str) -> Option<ErrorAcknowledged>,
	) {
		self.errors.retain(|(id, error)| match show(*id, error) {
			Some(ErrorAcknowledged) => false,
			None => true,
		});
	}

	pub fn show_errors(&mut self, ctx: &Context) {
		self.show_errors_inner(|id, error| {
			let response = egui::Window::new("Error").id(id).show(ctx, |ui| {
				ui.heading("An error occurred.");
				ui.label(error);
				ui.vertical_centered(|ui| ui.button("Ok").clicked()).inner
			});

			response
				.and_then(|response| response.inner)
				.unwrap_or(false)
				.then_some(ErrorAcknowledged)
		});
	}

	pub fn current_path(&self) -> Option<&Path> {
		self.current.as_ref().map(|open| &*open.path)
	}

	pub fn next_path(&mut self, args: NextPath) {
		self.actor.next_path(args);
	}

	pub fn delete_file(&mut self, file: Arc<Path>) {
		self.actor.delete_file(file);
	}

	pub fn handle_actor_responses(&mut self) {
		while let Some(response) = self.actor.poll_response() {
			let response = match response {
				Ok(response) => response,
				Err(error) => {
					self.push_error(error.to_string());
					continue;
				}
			};
			match response {
				Response::LoadImage(LoadedImage { path, image }) => {
					let inner = image.map(|image| {
						let play_state = image.make_play_state();
						OpenImageInner {
							play_state,
							image,
							zoom: crate::widgets::image::Zoom::default(),
						}
					});
					self.current = Some(OpenImage { inner, path });
				}
				Response::NoOp => (),
			}
		}
	}
}
