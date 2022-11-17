use std::ops::Range;

use egui::{NumExt as _, Rect, ScrollArea, Ui};

pub trait ShowColumnsExt {
	fn show_columns(
		self,
		ui: &mut Ui,
		item_width_without_spacing: f32,
		total_items: usize,
		add_contents: impl FnOnce(&mut Ui, Range<usize>),
	);
}

impl ShowColumnsExt for ScrollArea {
	// based on the `show_rows` implementation in egui.
	fn show_columns(
		self,
		ui: &mut Ui,
		item_width_without_spacing: f32,
		total_items: usize,
		add_contents: impl FnOnce(&mut Ui, Range<usize>),
	) {
		let spacing = ui.spacing().item_spacing;
		let item_width_with_spacing = item_width_without_spacing + spacing.x;
		self.show_viewport(ui, |ui, viewport| {
			ui.set_width({
				let total_items_f: f32 = az::cast(total_items);
				let including_last_padding = item_width_with_spacing * total_items_f;
				let width = including_last_padding - spacing.x;
				width.at_least(0.0)
			});

			let min_col = az::cast::<_, usize>((viewport.min.x / item_width_with_spacing).floor());
			let max_col = az::cast::<_, usize>((viewport.max.x / item_width_with_spacing).ceil()) + 1;
			let max_col = max_col.at_most(total_items);

			let x_min = ui.max_rect().left() + az::cast::<_, f32>(min_col) * item_width_with_spacing;
			let x_max = ui.max_rect().left() + az::cast::<_, f32>(max_col) * item_width_with_spacing;

			let rect = Rect::from_x_y_ranges(x_min..=x_max, ui.max_rect().y_range());

			ui.allocate_ui_at_rect(rect, |ui| {
				ui.skip_ahead_auto_ids(min_col);
				ui.horizontal(|ui| {
					add_contents(ui, min_col..max_col);
				});
			});
		});
	}
}
