#[derive(Debug, Clone, Copy)]
pub struct Seconds(pub f32);

impl Seconds {
	pub fn advance(&mut self, elapsed: f32) -> bool {
		self.0 -= elapsed;
		self.is_over()
	}

	pub fn is_over(self) -> bool {
		self.0 < 0.0
	}
}

impl From<Seconds> for std::time::Duration {
	fn from(seconds: Seconds) -> Self {
		Self::from_secs_f32(seconds.0)
	}
}

impl From<image::Delay> for Seconds {
	fn from(delay: image::Delay) -> Self {
		let (numer, denom) = delay.numer_denom_ms();
		Self((az::cast::<_, f32>(numer) / az::cast::<_, f32>(denom)) * 0.001)
	}
}

impl std::fmt::Display for Seconds {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let (value, unit) = if self.0 < 0.000_001 {
			let nanos = self.0 / 0.000_000_001;
			(nanos, "ns")
		} else if self.0 < 0.001 {
			let micros = self.0 / 0.000_001;
			(micros, "us")
		} else if self.0 < 1.0 {
			let millis = self.0 / 0.001;
			(millis, "ms")
		} else {
			let seconds = self.0;
			(seconds, "s")
		};

		write!(formatter, "{value:.0} {unit}")
	}
}
