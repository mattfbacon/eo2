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
	DeleteFile {
		path: PathBuf,
		should_go_to_next: bool,
	},
}

pub enum NextPath {
	Some(PathBuf),
	NoOthers,
	NoFilesAtAll,
}

pub enum Response {
	LoadImage(PathBuf, ImageResult<image::Image>),
	NextPath(NextPath),
	NoOp,
}

#[derive(Debug, Clone, Copy)]
pub enum SendResult {
	Sent,
	AlreadyWaiting,
}

pub struct Actor {
	command_sender: mpsc::SyncSender<Command>,
	response_receiver: mpsc::Receiver<io::Result<Response>>,
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

	pub fn load_image(&mut self, path: PathBuf) -> SendResult {
		self.send(Command::LoadImage(path))
	}

	pub fn next_path(&mut self, path: PathBuf, direction: NextPathDirection) -> SendResult {
		self.send(Command::NextPath(path, direction))
	}

	pub fn delete_file(&mut self, path: PathBuf, should_go_to_next: bool) -> SendResult {
		self.send(Command::DeleteFile {
			path,
			should_go_to_next,
		})
	}
}

fn in_thread(
	egui_ctx: &egui::Context,
	command_receiver: &mpsc::Receiver<Command>,
	response_sender: &mpsc::SyncSender<io::Result<Response>>,
) {
	while let Ok(command) = command_receiver.recv() {
		let response = run_command(command, egui_ctx);
		response_sender.send(response).unwrap();
		egui_ctx.request_repaint();
	}
}

fn run_command(command: Command, egui_ctx: &egui::Context) -> io::Result<Response> {
	Ok(match command {
		Command::LoadImage(path) => {
			let loaded = load_image(egui_ctx, &path);
			Response::LoadImage(path, loaded)
		}
		Command::NextPath(current_path, direction) => {
			let next_path = next_path(&current_path, direction)?;
			Response::NextPath(next_path.map_or(NextPath::NoOthers, NextPath::Some))
		}
		Command::DeleteFile {
			path,
			should_go_to_next,
		} => {
			std::fs::remove_file(&path)?;
			if should_go_to_next {
				let next_path = next_path(&path, NextPathDirection::RIGHT)?;
				Response::NextPath(next_path.map_or(NextPath::NoFilesAtAll, NextPath::Some))
			} else {
				Response::NoOp
			}
		}
	})
}

fn load_image(egui_ctx: &egui::Context, path: &Path) -> ImageResult<image::Image> {
	image::Image::load(egui_ctx, path)
}
