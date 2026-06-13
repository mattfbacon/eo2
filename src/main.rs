#![deny(
	absolute_paths_not_starting_with_crate,
	future_incompatible,
	keyword_idents,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	non_ascii_idents,
	nonstandard_style,
	noop_method_call,
	rust_2018_idioms,
	unused_qualifications
)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

mod app;
mod args;
mod config;
mod duration;
mod error;
mod widgets;

fn main() -> Result<(), ()> {
	match main_() {
		Ok(()) => Ok(()),
		Err(error) => {
			error::show(error.0);
			Err(())
		}
	}
}

fn main_() -> Result<(), error::Stringed> {
	app::init_timezone();

	let args = args::load();
	let config = config::load()?;

	eframe::run_native(
		"Image Viewer",
		eframe::NativeOptions::default(),
		Box::new(move |cc| Ok(Box::new(app::App::new(args, config, cc)))),
	)
	.unwrap();

	Ok(())
}
