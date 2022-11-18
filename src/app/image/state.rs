use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::task::Poll;

use ::image::error::{ImageError, ImageResult};
use egui::Context;

use super::Image;
use crate::app::actor::{Actor, Response};
use crate::app::next_path::Direction as NextPathDirection;

struct OpenImage {
	status: ImageResult<()>,
	path: PathBuf,
}

pub enum NavigationMode {
	InDirectory,
	Specified { paths: Vec<PathBuf>, current: usize },
}

pub struct State {
	cache: HashMap<PathBuf, Image>,
	current: Option<OpenImage>,
	navigation_mode: NavigationMode,
	actor: Actor,
}

impl State {
	pub fn new(egui_ctx: Context, navigation_mode: NavigationMode) -> Self {
		Self {
			cache: HashMap::new(),
			current: None,
			navigation_mode,
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

	#[must_use = "must handle the Poll::Ready variant eagerly"]
	pub fn next_path(&mut self, direction: NextPathDirection) -> Poll<PathBuf> {
		match &mut self.navigation_mode {
			NavigationMode::InDirectory => {
				if let Some(path) = self.current_path() {
					self.actor.next_path(path.into(), direction);
				}
				Poll::Pending
			}
			NavigationMode::Specified { paths, current } => {
				*current = (*current + 1) % paths.len();
				let next_path = paths[*current].clone();
				Poll::Ready(next_path)
			}
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
