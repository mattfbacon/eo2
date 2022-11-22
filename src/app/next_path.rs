use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub enum SimpleDirection {
	Left,
	Right,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
	Simple(SimpleDirection),
	Random,
}

impl SimpleDirection {
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

	pub fn step(self, current: usize, num_items: usize) -> usize {
		match self {
			Self::Right => (current + 1) % num_items,
			Self::Left => current.checked_sub(1).unwrap_or(num_items - 1),
		}
	}
}

impl SimpleDirection {
	fn find_next(self, current_name: &str, dir: std::fs::ReadDir) -> io::Result<Option<String>> {
		let direction = self;
		let mut next_name: Option<String> = None;
		let mut wrapped_name: Option<String> = None;

		for entry in dir {
			let entry = entry?;

			if entry.file_type()?.is_dir() {
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

		Ok(next_name.or(wrapped_name))
	}
}

fn choose_random(current_name: &str, dir: std::fs::ReadDir) -> io::Result<Option<String>> {
	let mut entries = vec![];

	for entry in dir {
		let entry = entry?;

		if entry.file_type()?.is_dir() {
			continue;
		}

		let this_name = entry.file_name();

		if image::ImageFormat::from_path(&this_name).is_err() {
			continue;
		}

		let this_name = this_name.to_string_lossy().into_owned();

		if this_name == current_name {
			continue;
		}

		entries.push(this_name);
	}

	Ok(rand::seq::SliceRandom::choose(entries.as_slice(), &mut rand::thread_rng()).cloned())
}

impl Direction {
	pub const LEFT: Self = Self::Simple(SimpleDirection::Left);
	pub const RIGHT: Self = Self::Simple(SimpleDirection::Right);

	fn find_next(self, current_name: &str, dir: std::fs::ReadDir) -> io::Result<Option<String>> {
		match self {
			Self::Simple(simple) => simple.find_next(current_name, dir),
			Self::Random => choose_random(current_name, dir),
		}
	}

	pub fn step(self, current: usize, num_items: usize) -> usize {
		match self {
			Self::Simple(simple) => simple.step(current, num_items),
			Self::Random => {
				// subtract one from max, then add one to the generated if the value >= current, to exclude current.
				let rand = rand::Rng::gen_range(&mut rand::thread_rng(), 0..(num_items - 1));
				if rand >= current {
					rand + 1
				} else {
					rand
				}
			}
		}
	}
}

pub fn next_path(current_path: &Path, direction: Direction) -> io::Result<Option<PathBuf>> {
	let parent = current_path.parent().unwrap(/* path must have a parent because it must be a file, though it may be empty. */);
	let current_name = current_path.file_name().unwrap(/* ditto */).to_string_lossy();

	let readable_parent = if parent.as_os_str().is_empty() {
		".".as_ref()
	} else {
		parent
	};

	let next_name = direction.find_next(&current_name, readable_parent.read_dir()?)?;

	Ok(next_name.map(|next_name| parent.join(next_name)))
}
