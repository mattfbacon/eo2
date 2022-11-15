use std::path::PathBuf;

use figment::providers::{Format as _, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	#[serde(default)]
	pub background: Background,
	#[serde(default)]
	pub show_sidebar: bool,
	#[serde(default)]
	pub show_frames: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum Background {
	#[default]
	Default,
	Dark,
	Light,
	DarkChecker,
	LightChecker,
}

fn config_path() -> PathBuf {
	directories_next::ProjectDirs::from("nz", "felle", "eo2")
		.expect("getting configuration path")
		.config_dir()
		.to_owned()
		.join("config.toml")
}

impl Config {
	pub fn load() -> Self {
		Figment::new()
			.merge(Toml::file(config_path()))
			.extract()
			.expect("loading configuration")
	}

	/*
	pub fn save(&self) {
		std::fs::write(
			config_path(),
			toml::to_vec(self).expect("serializing configuration"),
		)
		.expect("writing configuration");
	}
	*/
}

pub fn load() -> Config {
	Config::load()
}
