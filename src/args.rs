use std::path::PathBuf;

/// View images
#[derive(argh::FromArgs)]
pub struct Args {
	/// the image(s) to open
	///
	/// if multiple images are specified, only these images will be used when moving left and right, rather than all the images in the directory of the initial image.
	#[argh(positional)]
	pub paths: Vec<PathBuf>,
}

pub fn load() -> Args {
	argh::from_env()
}
