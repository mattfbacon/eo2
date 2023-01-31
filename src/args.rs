use std::path::{Path, PathBuf};
use std::sync::Arc;

/// View images
#[derive(argh::FromArgs)]
pub struct Args {
	/// the image(s) to open
	///
	/// if multiple images are specified, only these images will be used when moving left and right, rather than all the images in the directory of the initial image.
	#[argh(positional, from_str_fn(via_pathbuf))]
	pub paths: Vec<Arc<Path>>,
}

#[allow(clippy::unnecessary_wraps)] // required for `argh` interface
fn via_pathbuf(s: &str) -> Result<Arc<Path>, String> {
	Ok(PathBuf::from(s).into())
}

pub fn load() -> Args {
	argh::from_env()
}
