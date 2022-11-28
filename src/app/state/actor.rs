// In this actor, rather than using the typical pattern of passing "response" channels in the commands, we have a single response channel.
// This makes it easier to handle responses in the UI code, since we only need to poll one channel rather than a dynamic number of them.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::{io, thread};

use ::image::error::ImageResult;

use crate::app::image;
use crate::app::next_path::{next_path, Direction as NextPathDirection};

enum Command {
	LoadImage(PathBuf),
	NextPath(PathBuf, NextPathDirection),
	TrashFile(PathBuf),
}

pub enum NextPath {
	Some(PathBuf),
	NoOthers,
	NoFilesAtAll,
}

/// Variants in `Response` are named according to their corresponding `Command`, except where otherwise noted.
pub enum Response {
	LoadImage(PathBuf, ImageResult<image::Image>),
	/// This variant is used for the `ToTrash` command.
	NextPath(io::Result<NextPath>),
}

#[derive(Debug, Clone, Copy)]
pub enum SendResult {
	Sent,
	AlreadyWaiting,
}

pub struct Actor {
	command_sender: mpsc::SyncSender<Command>,
	response_receiver: mpsc::Receiver<Response>,
	waiting: bool,
}

impl Actor {
	pub fn spawn(egui_ctx: egui::Context) -> Self {
		let (command_sender, command_receiver) = mpsc::sync_channel(1);
		let (response_sender, response_receiver) = mpsc::sync_channel(1);
		// detach thread
		thread::spawn(move || in_thread(&egui_ctx, &command_receiver, &response_sender));
		Self {
			command_sender,
			response_receiver,
			waiting: false,
		}
	}

	pub fn waiting(&self) -> bool {
		self.waiting
	}

	pub fn poll_response(&mut self) -> Option<Response> {
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

	pub fn load_image(&mut self, path: PathBuf) -> SendResult {
		self.send(Command::LoadImage(path))
	}

	pub fn next_path(&mut self, path: PathBuf, direction: NextPathDirection) -> SendResult {
		self.send(Command::NextPath(path, direction))
	}

	pub fn trash_file(&mut self, path: PathBuf) -> SendResult {
		self.send(Command::TrashFile(path))
	}
}

fn in_thread(
	egui_ctx: &egui::Context,
	command_receiver: &mpsc::Receiver<Command>,
	response_sender: &mpsc::SyncSender<Response>,
) {
	while let Ok(command) = command_receiver.recv() {
		let response = match command {
			Command::LoadImage(path) => {
				let loaded = load_image(egui_ctx, &path);
				Response::LoadImage(path, loaded)
			}
			Command::NextPath(current_path, direction) => Response::NextPath(
				next_path(&current_path, direction)
					.map(|opt_path| opt_path.map_or(NextPath::NoOthers, NextPath::Some)),
			),
			Command::TrashFile(current_path) => Response::NextPath(to_trash(&current_path)),
		};
		response_sender.send(response).unwrap();
		egui_ctx.request_repaint();
	}
}

fn to_trash(current_path: &Path) -> io::Result<NextPath> {
	std::fs::remove_file(current_path)?;
	next_path(current_path, NextPathDirection::RIGHT)
		.map(|opt_path| opt_path.map_or(NextPath::NoFilesAtAll, NextPath::Some))
}

fn load_image(egui_ctx: &egui::Context, path: &Path) -> ImageResult<image::Image> {
	image::Image::load(egui_ctx, path)
}
