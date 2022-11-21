use egui::{Key, Response, TextEdit, TextStyle, Ui, Widget};

use crate::duration::Duration;

pub struct UnitInput<GS> {
	get_set: GS,
}

impl<GS: FnMut(Option<&str>) -> String> UnitInput<GS> {
	pub fn new(get_set: GS) -> Self {
		Self { get_set }
	}
}

// kinda cheating
impl UnitInput<()> {
	pub fn size(size: &mut usize) -> UnitInput<impl '_ + FnMut(Option<&str>) -> String> {
		UnitInput::new(move |set| {
			if let Some(set) = set {
				if let Some(parsed) = parse_size(set) {
					*size = parsed;
				}
			}

			humansize::format_size(*size, humansize::DECIMAL)
		})
	}

	pub fn duration(duration: &mut Duration) -> UnitInput<impl '_ + FnMut(Option<&str>) -> String> {
		UnitInput::new(move |set| {
			if let Some(set) = set {
				if let Ok(parsed) = set.parse() {
					*duration = parsed;
				}
			}

			duration.to_string()
		})
	}
}

fn parse_size(raw: &str) -> Option<usize> {
	let amount_end = raw
		.bytes()
		.position(|ch| !ch.is_ascii_digit() && ch != b'-' && ch != b'.')
		.unwrap_or(raw.len());
	let (amount, unit) = raw.split_at(amount_end);
	let unit = unit.trim_start();

	let amount = amount.parse::<f32>().ok()?;
	let scale = match unit.to_ascii_lowercase().as_str() {
		"b" => 1.0,
		"kb" => 1_000.0,
		"mb" => 1_000_000.0,
		// empty = gb
		"" | "gb" => 1_000_000_000.0,
		"tb" => 1_000_000_000_000.0,
		"pb" => 1_000_000_000_000_000.0,
		"kib" => 1024.0,
		"mib" => 1024.0 * 1024.0,
		"gib" => 1024.0 * 1024.0 * 1024.0,
		"tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
		"pib" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
		_ => return None,
	};
	let amount = amount * scale;

	az::checked_cast(amount)
}

impl<GS: FnMut(Option<&str>) -> String> Widget for UnitInput<GS> {
	fn ui(mut self, ui: &mut Ui) -> Response {
		let kb_edit_id = ui.id().with("kb_edit");

		let mut buffer = if ui.memory().has_focus(kb_edit_id) {
			std::mem::take(
				ui.memory()
					.data
					.get_temp_mut_or_insert_with(kb_edit_id, || (self.get_set)(None)),
			)
		} else {
			(self.get_set)(None)
		};

		let response = TextEdit::singleline(&mut buffer)
			.id(kb_edit_id)
			.font(TextStyle::Monospace)
			.desired_width(ui.spacing().interact_size.x * 2.0)
			.ui(ui);

		if response.changed() {
			// don't set `buffer` to the result, since we want to remember the user's input until focus is lost.
			(self.get_set)(Some(&buffer));
		}

		if response.has_focus() {
			if ui.input().key_pressed(Key::Enter) {
				ui.memory().surrender_focus(kb_edit_id);
				ui.memory().data.remove::<String>(kb_edit_id);
			} else {
				*ui.memory().data.get_temp_mut_or(kb_edit_id, String::new()) = buffer;
			}
		} else if response.lost_focus() {
			ui.memory().data.remove::<String>(kb_edit_id);
		}

		// propagating `response.changed()`
		response
	}
}
