#[derive(Debug, Clone, Copy)]
pub struct Seconds {
	micros: u32,
}

impl Seconds {
	pub fn new_secs_f32(secs: f32) -> Self {
		Self {
			micros: az::cast(secs * 1_000_000.0),
		}
	}

	pub fn new_millis_f32(millis: f32) -> Self {
		Self {
			micros: az::cast(millis * 1_000.0),
		}
	}

	pub const fn new_secs(secs: u32) -> Self {
		Self {
			micros: secs * 1_000_000,
		}
	}

	pub fn advance(&mut self, elapsed_secs: f32) -> bool {
		self.micros = self
			.micros
			.saturating_sub(Self::new_secs_f32(elapsed_secs).micros);
		self.is_over()
	}

	pub fn is_over(self) -> bool {
		self.micros == 0
	}
}

impl From<Seconds> for std::time::Duration {
	fn from(seconds: Seconds) -> Self {
		Self::from_micros(seconds.micros.into())
	}
}

impl From<image::Delay> for Seconds {
	fn from(delay: image::Delay) -> Self {
		let (numer, denom) = delay.numer_denom_ms();
		Self::new_millis_f32(az::cast::<_, f32>(numer) / az::cast::<_, f32>(denom))
	}
}

impl std::fmt::Display for Seconds {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let (value, unit) = if self.micros < 1_000 {
			(self.micros, "us")
		} else if self.micros < 1_000_000 {
			(self.micros / 1_000, "ms")
		} else {
			(self.micros / 1_000_000, "s")
		};

		write!(formatter, "{value:.0} {unit}")
	}
}
