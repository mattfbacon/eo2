use std::path::PathBuf;

/// View images
#[derive(argh::FromArgs)]
pub struct Args {
	/// the image to open
	#[argh(positional)]
	pub path: PathBuf,
}

pub fn load() -> Args {
	argh::from_env()
}
