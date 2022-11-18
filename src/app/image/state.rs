use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ::image::error::{ImageError, ImageResult};
use egui::Context;

use super::Image;
use crate::app::actor::{Actor, Response};
use crate::app::next_path::Direction as NextPathDirection;

struct OpenImage {
	status: ImageResult<()>,
	path: PathBuf,
}

pub struct State {
	cache: HashMap<PathBuf, Image>,
	current: Option<OpenImage>,
	actor: Actor,
}

impl State {
	pub fn new(egui_ctx: Context) -> Self {
		Self {
			cache: HashMap::new(),
			current: None,
			actor: Actor::spawn(egui_ctx),
		}
	}

	pub fn waiting(&self) -> bool {
		self.actor.waiting()
	}

	pub fn current_path(&self) -> Option<&Path> {
		self.current.as_ref().map(|open| &*open.path)
	}

	pub fn current(&self) -> Option<Result<&Image, &ImageError>> {
		self.current.as_ref().map(|open| {
			open
				.status
				.as_ref()
				.map(|()| self.cache.get(&open.path).unwrap())
		})
	}

	pub fn current_mut(&mut self) -> Option<Result<&mut Image, &ImageError>> {
		self.current.as_ref().map(|open| {
			open
				.status
				.as_ref()
				.map(|()| self.cache.get_mut(&open.path).unwrap())
		})
	}

	pub fn open(&mut self, path: PathBuf) {
		if self.cache.contains_key(&path) {
			self.current = Some(OpenImage {
				path,
				status: Ok(()),
			});
		} else {
			self.actor.load_image(path);
		}
	}

	pub fn next_path(&mut self, direction: NextPathDirection) {
		if let Some(path) = self.current_path() {
			self.actor.next_path(path.into(), direction);
		}
	}

	pub fn handle_actor_responses(&mut self) {
		while let Some(response) = self.actor.poll_response() {
			match response {
				Response::LoadImage(path, loaded) => {
					let status = loaded.map(|image| {
						self.cache.insert(path.clone(), image);
					});
					self.current = Some(OpenImage { status, path });
				}
				Response::NextPath(next) => match next {
					Ok(Some(next)) => self.open(next),
					Ok(None) | Err(..) => {
						// TODO better way of handling Err? a dialog shown to the user?
					}
				},
			}
		}
	}
}
