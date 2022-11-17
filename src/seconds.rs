use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "SerdeRepr", into = "SerdeRepr")]
pub struct Seconds {
	micros: u32,
}

#[derive(Serialize, Deserialize)]
struct SerdeRepr(f32);

impl From<Seconds> for SerdeRepr {
	fn from(seconds: Seconds) -> Self {
		Self(seconds.as_secs_f32())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum FromStrError {
	#[error(transparent)]
	Float(#[from] std::num::ParseFloatError),
	#[error(transparent)]
	OutOfRange(#[from] OutOfRange),
}

impl FromStr for Seconds {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let value = s.parse().map_err(Self::Err::Float)?;
		Ok(Self::new_secs_f32(value)?)
	}
}

impl TryFrom<SerdeRepr> for Seconds {
	type Error = OutOfRange;

	fn try_from(repr: SerdeRepr) -> Result<Self, Self::Error> {
		Self::new_secs_f32(repr.0)
	}
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("value out of range")]
pub struct OutOfRange;

impl Seconds {
	pub const MAX: Self = Self { micros: u32::MAX };

	pub fn new_secs_f32(secs: f32) -> Result<Self, OutOfRange> {
		az::checked_cast(secs * 1_000_000.0)
			.ok_or(OutOfRange)
			.map(Self::new_micros)
	}

	pub fn as_secs_f32(self) -> f32 {
		az::cast::<_, f32>(self.micros) / 1_000_000.0
	}

	pub fn new_millis_f32(millis: f32) -> Result<Self, OutOfRange> {
		az::checked_cast(millis * 1_000.0)
			.ok_or(OutOfRange)
			.map(Self::new_micros)
	}

	pub fn new_micros_f32(micros: f32) -> Result<Self, OutOfRange> {
		az::checked_cast(micros)
			.ok_or(OutOfRange)
			.map(Self::new_micros)
	}

	pub const fn new_micros(micros: u32) -> Self {
		Self { micros }
	}

	pub const fn new_secs(secs: u32) -> Result<Self, OutOfRange> {
		// like this to be const
		match secs.checked_mul(1_000_000) {
			Some(micros) => Ok(Self::new_micros(micros)),
			None => Err(OutOfRange),
		}
	}

	/// Subtract `elapsed_secs` from the current value.
	/// Return whether the duration is elapsed after subtracting (same as `is_over`).
	pub fn advance(&mut self, elapsed_secs: f32) -> bool {
		self.micros = self
			.micros
			.saturating_sub(Self::new_secs_f32(elapsed_secs).unwrap().micros);
		self.is_over()
	}

	/// Whether this duration has elapsed.
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
		Self::new_millis_f32(az::cast::<_, f32>(numer) / az::cast::<_, f32>(denom)).unwrap()
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
