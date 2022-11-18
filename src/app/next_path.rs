use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
	Left,
	Right,
}

impl Direction {
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
}

pub fn next_path(current_path: &Path, direction: Direction) -> io::Result<Option<PathBuf>> {
	let parent = current_path.parent().unwrap(/* path must have a parent because it must be a file, though it may be empty. */);
	let current_name = current_path.file_name().unwrap(/* ditto */).to_string_lossy();

	let mut next_name: Option<String> = None;
	let mut wrapped_name: Option<String> = None;

	let readable_parent = if parent.as_os_str().is_empty() {
		".".as_ref()
	} else {
		parent
	};
	for entry in readable_parent.read_dir()? {
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

	let next_name = next_name.or(wrapped_name);
	Ok(next_name.map(|next_name| parent.join(next_name)))
}
