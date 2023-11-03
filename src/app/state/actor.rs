// In this actor, rather than using the typical pattern of passing "response" channels in the commands, we have a single response channel.
// This makes it easier to handle responses in the UI code, since we only need to poll one channel rather than a dynamic number of them.

use std::hash::BuildHasherDefault;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::{io, thread};

use clru::{CLruCache, CLruCacheConfig};
use image::error::ImageResult;
use rustc_hash::FxHasher;

use crate::app::image::Image;
use crate::app::next_path;

#[derive(Debug)]
pub enum NextPathMode {
	Simple,
	Random,
}

impl NextPathMode {
	fn with_random_seed(self, seed: u64) -> next_path::Mode {
		match self {
			Self::Simple => next_path::Mode::Simple,
			Self::Random => next_path::Mode::Random { seed },
		}
	}
}

#[derive(Debug)]
pub struct NextPath {
	pub direction: next_path::Direction,
	pub mode: NextPathMode,
}

impl NextPath {
	pub const RIGHT: Self = Self {
		direction: next_path::Direction::Right,
		mode: NextPathMode::Simple,
	};

	pub const LEFT: Self = Self {
		direction: next_path::Direction::Left,
		mode: NextPathMode::Simple,
	};

	pub const RANDOM: Self = Self {
		direction: next_path::Direction::Right,
		mode: NextPathMode::Random,
	};

	fn with_random_seed(self, seed: u64) -> next_path::NextPath {
		next_path::NextPath {
			direction: self.direction,
			mode: self.mode.with_random_seed(seed),
		}
	}
}

#[derive(Debug)]
enum Command {
	NextPath(NextPath),
	DeleteFile(Arc<Path>),
}

pub struct LoadedImage {
	pub path: Arc<Path>,
	pub image: ImageResult<Arc<Image>>,
}

#[must_use = "responses must be handled"]
pub enum Response {
	LoadImage(LoadedImage),
	NoOp,
}

#[derive(Debug, Clone, Copy)]
pub enum SendResult {
	Sent,
	AlreadyWaiting,
}

#[derive(Debug)]
pub enum NavigationMode {
	InDirectory {
		current: Arc<Path>,
	},
	Specified {
		paths: Vec<Arc<Path>>,
		current: usize,
	},
	Empty,
}

impl NavigationMode {
	pub fn specified(paths: Vec<Arc<Path>>) -> Self {
		Self::Specified { paths, current: 0 }
	}

	fn current_path(&self) -> Option<&Arc<Path>> {
		match self {
			Self::InDirectory { current } => Some(current),
			Self::Specified { paths, current } => Some(&paths[*current]),
			Self::Empty => None,
		}
	}

	fn next_path(&mut self, args: next_path::NextPath) -> io::Result<Option<&Arc<Path>>> {
		Ok(match self {
			Self::InDirectory { current } => next_path::next_in_directory(current, args)?.map(|next| {
				*current = next.into();
				&*current
			}),
			Self::Specified { paths, current } => {
				next_path::next_in_list(paths.iter().map(|path| &**path), &paths[*current], args).map(
					|next| {
						*current = next;
						&paths[next]
					},
				)
			}
			Self::Empty => None,
		})
	}
}

pub struct Handle {
	command_sender: mpsc::SyncSender<Command>,
	response_receiver: mpsc::Receiver<io::Result<Response>>,
	waiting: bool,
}

impl Handle {
	pub fn spawn(
		egui_ctx: egui::Context,
		navigation_mode: NavigationMode,
		cache_size: NonZeroUsize,
	) -> Self {
		let (command_sender, command_receiver) = mpsc::sync_channel(1);
		let (response_sender, response_receiver) = mpsc::sync_channel(1);
		thread::spawn(move || {
			let actor = Actor {
				bridge: Bridge {
					egui_ctx,
					command_receiver,
					response_sender,
				},
				state: State {
					cache: CLruCache::with_config(
						CLruCacheConfig::new(cache_size)
							.with_hasher(BuildHasherDefault::default())
							.with_scale(ImageSizeWeight),
					),
					navigation_mode,
					random_seed: rand::random(),
				},
			};
			actor.run();
		});
		Self {
			command_sender,
			response_receiver,
			// waiting for initial LoadImage
			waiting: true,
		}
	}

	pub fn waiting(&self) -> bool {
		self.waiting
	}

	pub fn poll_response(&mut self) -> Option<io::Result<Response>> {
		match self.response_receiver.try_recv() {
			Ok(response) => {
				self.waiting = false;
				Some(response)
			}
			Err(mpsc::TryRecvError::Empty) => None,
			Err(mpsc::TryRecvError::Disconnected) => panic!("actor disconnected"),
		}
	}

	fn send(&mut self, command: Command) -> SendResult {
		if self.waiting {
			return SendResult::AlreadyWaiting;
		}
		self
			.command_sender
			.send(command)
			.expect("actor disconnected");
		self.waiting = true;
		SendResult::Sent
	}

	pub fn next_path(&mut self, args: NextPath) -> SendResult {
		self.send(Command::NextPath(args))
	}

	pub fn delete_file(&mut self, file: Arc<Path>) -> SendResult {
		self.send(Command::DeleteFile(file))
	}
}

struct Bridge {
	egui_ctx: egui::Context,
	command_receiver: mpsc::Receiver<Command>,
	response_sender: mpsc::SyncSender<io::Result<Response>>,
}

struct ImageSizeWeight;

impl clru::WeightScale<Arc<Path>, Arc<Image>> for ImageSizeWeight {
	fn weight(&self, _path: &Arc<Path>, image: &Arc<Image>) -> usize {
		image.size_in_memory()
	}
}

struct State {
	navigation_mode: NavigationMode,
	cache: CLruCache<Arc<Path>, Arc<Image>, BuildHasherDefault<FxHasher>, ImageSizeWeight>,
	random_seed: u64,
}

impl State {
	fn current_path(&self) -> Option<&Arc<Path>> {
		self.navigation_mode.current_path()
	}

	fn next_path(&mut self, args: NextPath) -> io::Result<Option<&Arc<Path>>> {
		self
			.navigation_mode
			.next_path(args.with_random_seed(self.random_seed))
	}
}

struct Actor {
	bridge: Bridge,
	state: State,
}

impl Actor {
	fn send_response(&self, response: io::Result<Response>) {
		self.bridge.response_sender.send(response).unwrap();
		self.bridge.egui_ctx.request_repaint();
	}

	fn run(mut self) {
		self.load_initial_image();

		while let Ok(command) = self.bridge.command_receiver.recv() {
			let response = self.run_command(command);
			self.send_response(response);
		}
	}

	fn load_initial_image(&mut self) {
		let response = match &self.state.navigation_mode.current_path() {
			Some(current_path) => self.load_image(Arc::clone(current_path)),
			None => Response::NoOp,
		};
		self.send_response(Ok(response));
	}

	fn load_image_(&mut self, path: &Arc<Path>) -> ImageResult<Arc<Image>> {
		Ok(if let Some(cached) = self.state.cache.get(path) {
			Arc::clone(cached)
		} else {
			let image = Arc::new(Image::load(&self.bridge.egui_ctx, path)?);
			_ = self
				.state
				.cache
				.put_with_weight(Arc::clone(path), Arc::clone(&image));
			image
		})
	}

	fn load_image(&mut self, path: Arc<Path>) -> Response {
		let image = self.load_image_(&path);
		Response::LoadImage(LoadedImage { path, image })
	}

	fn next_path(&mut self, args: NextPath) -> io::Result<Response> {
		let Some(next_path) = self.state.next_path(args)? else {
			return Ok(Response::NoOp);
		};
		let next_path = Arc::clone(next_path);
		Ok(self.load_image(next_path))
	}

	fn run_command(&mut self, command: Command) -> io::Result<Response> {
		match command {
			Command::NextPath(direction) => self.next_path(direction),
			Command::DeleteFile(path) => {
				std::fs::remove_file(&path)?;
				let should_go_to_next = Some(&*path) == self.state.current_path().map(|path| &**path);
				if should_go_to_next {
					self.next_path(NextPath::RIGHT)
				} else {
					Ok(Response::NoOp)
				}
			}
		}
	}
}
