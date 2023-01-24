use std::num::NonZeroUsize;
use std::path::PathBuf;

use eframe::Theme;
use egui::ComboBox;
use figment::providers::{Format as _, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

use crate::duration::Duration;
use crate::widgets;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	pub theme: Option<Theme>,
	#[serde(default)]
	pub show_sidebar: bool,
	#[serde(default)]
	pub show_frames: bool,
	#[serde(default = "default_cache_size")]
	pub cache_size: NonZeroUsize,
	#[serde(default)]
	pub background: Background,
	#[serde(default)]
	pub slideshow: Slideshow,
}

fn default_cache_size() -> NonZeroUsize {
	NonZeroUsize::new(1024 * 1024 * 1024).unwrap()
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default)]
pub struct Background {
	#[serde(default)]
	pub checkered: bool,
	#[serde(default)]
	pub color: BackgroundColor,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundColor {
	#[default]
	Default,
	Dark,
	Light,
}

impl BackgroundColor {
	pub fn repr(self) -> &'static str {
		match self {
			Self::Default => "Default",
			Self::Dark => "Dark",
			Self::Light => "Light",
		}
	}

	const VARIANTS: &[Self] = &[Self::Default, Self::Dark, Self::Light];
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Slideshow {
	#[serde(default = "default_interval")]
	pub interval: Duration,
	#[serde(default = "default_shuffle")]
	pub shuffle: bool,
}

impl Default for Slideshow {
	fn default() -> Self {
		Self {
			interval: default_interval(),
			shuffle: default_shuffle(),
		}
	}
}

fn default_interval() -> Duration {
	Duration::new_secs(5).unwrap()
}

fn default_shuffle() -> bool {
	false
}

impl Slideshow {
	fn ui(&mut self, ui: &mut egui::Ui) {
		widgets::KeyValue::new("config-slideshow-kv").show(ui, |mut rows| {
			rows.row("Interval", |ui| {
				/*
				let mut secs = self.interval.as_secs_f32();
				let widget = egui::DragValue::new(&mut secs)
					.speed(0.01)
					.suffix(" s")
					.clamp_range(0.001..=Duration::MAX.as_secs_f32());
					*/
				ui.add(crate::widgets::UnitInput::duration(&mut self.interval));
			});
			rows.row("Shuffle", |ui| ui.checkbox(&mut self.shuffle, ""));
		});
	}
}

fn config_path() -> PathBuf {
	directories_next::ProjectDirs::from("nz", "felle", "eo2")
		.expect("getting configuration path")
		.config_dir()
		.to_owned()
		.join("config.toml")
}

impl Background {
	fn ui(&mut self, ui: &mut egui::Ui) {
		widgets::KeyValue::new("config-background-kv").show(ui, |mut rows| {
			rows.row("Color", |ui| {
				ComboBox::from_id_source("config-background-color-combo")
					.selected_text(self.color.repr())
					.show_ui(ui, |ui| {
						for &variant in BackgroundColor::VARIANTS {
							ui.selectable_value(&mut self.color, variant, variant.repr());
						}
					})
			});
			rows.row("Checkered", |ui| ui.checkbox(&mut self.checkered, ""));
		});
	}
}

impl Config {
	pub fn load() -> figment::error::Result<Self> {
		Figment::new().merge(Toml::file(config_path())).extract()
	}

	pub fn save(&self) -> std::io::Result<()> {
		std::fs::write(
			config_path(),
			toml::to_string(self)
				.expect("serializing configuration")
				.as_bytes(),
		)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) {
		widgets::KeyValue::new("config-kv").show(ui, |mut rows| {
			rows.row("Background", |ui| {
				self.background.ui(ui);
			});
			rows.row("Color Scheme", |ui| {
				self.light_dark_toggle_button(ui);
			});
			rows.row("Slideshow", |ui| {
				self.slideshow.ui(ui);
			});
			rows.row("Cache Size", |ui| {
				let mut size = self.cache_size.get();
				if ui.add(crate::widgets::UnitInput::size(&mut size)).changed() {
					if let Some(nz) = NonZeroUsize::new(size) {
						self.cache_size = nz;
					}
				}
			});
		});
	}

	pub fn light_dark_toggle_button(&mut self, ui: &mut egui::Ui) {
		if let Some(new_visuals) = ui.ctx().style().visuals.light_dark_small_toggle_button(ui) {
			self.theme = Some(if new_visuals.dark_mode {
				Theme::Dark
			} else {
				Theme::Light
			});
			ui.ctx().set_visuals(new_visuals);
		}
	}
}

pub fn load() -> figment::error::Result<Config> {
	Config::load()
}
