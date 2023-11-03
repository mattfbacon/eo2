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
Right Arrow, n | Go to next
Left Arrow, p, Shift-n | Go to previous
Ctrl-Shift-i | Toggle internal state window
c | Toggle settings
f | Toggle fullscreen
i | Toggle info panel
s | Toggle slideshow

The bindings that move between images normally respect slideshows, i.e., if there is an active slideshow and shuffle is enabled, the keys will move with the same randomness. To override this, use Alt.

## License

AGPL-3.0-or-later
