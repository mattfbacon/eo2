use std::path::PathBuf;

use eframe::Theme;
use egui::ComboBox;
use figment::providers::{Format as _, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};

use crate::seconds::Seconds;
use crate::widgets;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	pub theme: Option<Theme>,
	#[serde(default)]
	pub show_sidebar: bool,
	#[serde(default)]
	pub show_frames: bool,
	#[serde(default)]
	pub background: Background,
	#[serde(default)]
	pub slideshow: Slideshow,
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
	pub interval: Seconds,
}

impl Default for Slideshow {
	fn default() -> Self {
		Self {
			interval: default_interval(),
		}
	}
}

fn default_interval() -> Seconds {
	Seconds::new_secs(5).unwrap()
}

impl Slideshow {
	fn ui(&mut self, ui: &mut egui::Ui) {
		widgets::KeyValue::new("config-slideshow-kv").show(ui, |mut rows| {
			rows.row("Interval", |ui| {
				let mut secs = self.interval.as_secs_f32();
				let widget = egui::DragValue::new(&mut secs)
					.speed(0.01)
					.suffix(" s")
					.clamp_range(0.001..=Seconds::MAX.as_secs_f32());
				let response = ui.add(widget);
				if response.changed() {
					if let Ok(new) = Seconds::new_secs_f32(secs) {
						self.interval = new;
					}
				}
			});
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
	pub fn load() -> Self {
		Figment::new()
			.merge(Toml::file(config_path()))
			.extract()
			.expect("loading configuration")
	}

	pub fn save(&self) {
		std::fs::write(
			config_path(),
			toml::to_vec(self).expect("serializing configuration"),
		)
		.expect("writing configuration");
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

pub fn load() -> Config {
	Config::load()
}
