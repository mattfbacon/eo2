use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;

use clru::{CLruCache, CLruCacheConfig};
use egui::Context;
use image::error::ImageResult;
use xxhash_rust::xxh3::Xxh3Builder;

use self::actor::{Actor, Response};
use super::image::Image;
use crate::app::next_path::Direction as NextPathDirection;

pub mod actor;
pub mod play;

pub struct OpenImageInner {
	pub play_state: play::State,
	pub image: Rc<Image>,
	pub zoom: crate::widgets::image::Zoom,
}

pub struct OpenImage {
	pub inner: ImageResult<OpenImageInner>,
	pub path: PathBuf,
}

#[derive(Debug)]
pub enum NavigationMode {
	InDirectory,
	Specified { paths: Vec<PathBuf>, current: usize },
}

impl NavigationMode {
	pub fn specified(paths: Vec<PathBuf>) -> Self {
		Self::Specified { paths, current: 0 }
	}
}

struct ImageSizeWeight;

impl clru::WeightScale<PathBuf, Rc<Image>> for ImageSizeWeight {
	fn weight(&self, _path: &PathBuf, image: &Rc<Image>) -> usize {
		image.size_in_memory()
	}
}

static ERRORS_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct State {
	cache: CLruCache<PathBuf, Rc<Image>, Xxh3Builder, ImageSizeWeight>,
	pub current: Option<OpenImage>,
	navigation_mode: NavigationMode,
	actor: Actor,
	errors: Vec<(egui::Id, String)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ErrorAcknowledged;

impl State {
	pub fn new(egui_ctx: Context, cache_size: NonZeroUsize, navigation_mode: NavigationMode) -> Self {
		Self {
			cache: CLruCache::with_config(
				CLruCacheConfig::new(cache_size)
					.with_hasher(Xxh3Builder::new())
					.with_scale(ImageSizeWeight),
			),
			current: None,
			navigation_mode,
			actor: Actor::spawn(egui_ctx),
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

	pub fn open(&mut self, path: PathBuf) {
		if let Some(cached) = self.cache.get(&path) {
			let image = Rc::clone(cached);
			let play_state = image.make_play_state();
			let inner = OpenImageInner {
				play_state,
				image,
				zoom: Default::default(),
			};
			self.current = Some(OpenImage {
				path,
				inner: Ok(inner),
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
				*current = direction.step(*current, paths.len());
				let next_path = paths[*current].clone();
				Poll::Ready(next_path)
			}
		}
	}

	pub fn trash_current(&mut self) {
		if let Some(path) = self.current_path() {
			self.actor.trash_file(path.into());
		}
	}

	pub fn handle_actor_responses(&mut self) {
		while let Some(response) = self.actor.poll_response() {
			match response {
				Response::LoadImage(path, loaded) => {
					let inner = loaded.and_then(|image| {
						let play_state = image.make_play_state();
						let image = Rc::new(image);
						self
							.cache
							.put_with_weight(path.clone(), Rc::clone(&image))
							.map_err(|_| {
								use image::error::{ImageError, LimitError, LimitErrorKind};
								ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory))
							})?;
						Ok(OpenImageInner {
							play_state,
							image,
							zoom: Default::default(),
						})
					});
					self.current = Some(OpenImage { inner, path });
				}
				Response::NextPath(next) => match next {
					Ok(actor::NextPath::Some(next)) => self.open(next),
					Ok(actor::NextPath::NoOthers) => (),
					Ok(actor::NextPath::NoFilesAtAll) => {
						// not using `current_path` due to borrow granularity
						if let Some(current) = &self.current {
							_ = self.cache.pop(&current.path);
						}
						self.current = None;
					}
					Err(error) => self.push_error(error.to_string()),
				},
			}
		}
	}

	pub fn internal_ui(&mut self, ui: &mut egui::Ui) {
		use crate::widgets::KeyValue;

		KeyValue::new("image-state-internal-kv").show(ui, |mut rows| {
			rows.sub("image-state-internal-cache-kv", "Cache", |mut rows| {
				rows.row("Size", |ui| {
					ui.label(humansize::format_size(
						self.cache.weight(),
						humansize::DECIMAL,
					));
				});
				rows.row("Limit", |ui| {
					ui.label(humansize::format_size(
						self.cache.capacity(),
						humansize::DECIMAL,
					));
				});
				rows.row("Entries", |ui| {
					ui.vertical(|ui| {
						ui.label(self.cache.len().to_string());
						ui.collapsing("The entries (LRU order)", |ui| {
							egui::ScrollArea::vertical().show_rows(
								ui,
								egui::style::TextStyle::Body.resolve(ui.style()).size,
								self.cache.len(),
								|ui, range| {
									let std::ops::Range { start, end } = range;
									for (idx, (path, entry)) in
										range.zip(self.cache.iter().skip(start).take(end - start))
									{
										ui.label(format!(
											"{}. {:?}, {}x{}, {}, {} frames",
											idx + 1,
											path,
											entry.width,
											entry.height,
											humansize::format_size(entry.size_in_memory(), humansize::DECIMAL),
											entry.frames.len()
										));
									}
								},
							);
						});
					});
				});
				rows.row("Empty", |ui| {
					if ui.button("Empty").clicked() {
						self.cache.clear();
					}
				});
			});
			rows.row("Nav Mode", |ui| {
				ui.vertical(|ui| match &self.navigation_mode {
					NavigationMode::InDirectory => match self.current_path() {
						Some(path) => {
							ui.label(format!("All images in {path:?}"));
						}
						None => {
							ui.label("N/A, no images");
						}
					},
					NavigationMode::Specified { paths, current } => {
						ui.label(format!(
							"{} out of {} specified paths",
							current + 1,
							paths.len()
						));
						ui.collapsing("The paths", |ui| {
							egui::ScrollArea::vertical().show_rows(
								ui,
								egui::style::TextStyle::Body.resolve(ui.style()).size,
								paths.len(),
								|ui, range| {
									for idx in range {
										ui.label(format!("{}. {:?}", idx + 1, paths[idx]));
									}
								},
							);
						});
					}
				});
			});
			rows.row("Actor", |ui| {
				ui.label(if self.actor.waiting() {
					"Waiting"
				} else {
					"Ready"
				});
			});
		});
	}
}
