use crate::app::image::Image;
use crate::seconds::Seconds;

#[derive(Debug, Clone, Copy)]
pub struct CurrentFrame {
	pub idx: usize,
	pub remaining: Seconds,
}

impl CurrentFrame {
	pub fn new(remaining: impl Into<Seconds>) -> Self {
		Self::new_at(0, remaining.into())
	}

	pub fn new_at(idx: usize, remaining: impl Into<Seconds>) -> Self {
		Self {
			idx,
			remaining: remaining.into(),
		}
	}

	pub fn move_to(&mut self, idx: usize, remaining: impl Into<Seconds>) {
		*self = Self::new_at(idx, remaining.into());
	}

	pub fn advance(
		&mut self,
		elapsed: Seconds,
		num_frames: usize,
		mut get_frame_time: impl FnMut(usize) -> Seconds,
	) {
		// note: this intentionally never advances more than one frame
		if self.remaining.advance(elapsed) {
			self.idx = (self.idx + 1) % num_frames;
			self.remaining = get_frame_time(self.idx);
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum State {
	Animated {
		current_frame: CurrentFrame,
		playing: bool,
	},
	Single,
}

impl Image {
	pub fn make_play_state(&self) -> State {
		if self.is_animated() {
			let current_delay = self.frames[0].1;
			State::Animated {
				current_frame: CurrentFrame::new(current_delay),
				playing: true,
			}
		} else {
			State::Single
		}
	}
}
