use egui::Vec2;

pub use self::columns::ShowColumnsExt;
pub use self::image::Image;
pub use self::image_button::ImageButton;
pub use self::key_value::KeyValue;
pub use self::unit_input::UnitInput;

pub mod columns;
pub mod image;
pub mod image_button;
pub mod key_value;
pub mod unit_input;

fn image_size(actual: Vec2, max: Vec2) -> Vec2 {
	assert!(!actual.any_nan(), "NaN encountered");

	if actual.x < max.x && actual.y < max.y {
		actual
	} else {
		let x_ratio = max.x / actual.x;
		let y_ratio = max.y / actual.y;
		actual * std::cmp::min_by(x_ratio, y_ratio, |a, b| a.partial_cmp(b).unwrap())
	}
}
