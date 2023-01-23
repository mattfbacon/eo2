# eo2

An image viewer written in Rust using egui.

Aims to provide all the features that I personally want.

## Non-exhaustive List of Features

- Slideshow
	- Uses natural ordering, meaning numbered files are ordered properly even without zero-padding.
	- Random mode
- Animated images
	- Frames menu
- Zoom and panning
- Info panel

## Configuration

Via `~/.config/eo2/config.toml` and the settings panel in the UI.

See the settings panel in the UI for a list of configuration.

## Keybindings

Binding | Action
-:|:-
Right Arrow | Go to next
Left Arrow | Go to previous
Ctrl-Shift-I | Toggle internal state window

## License

AGPL-3.0-or-later
