use std::str::FromStr;

use serde::{de, ser};

#[derive(Debug, Clone, Copy)]
pub struct Duration {
	micros: u32,
}

impl ser::Serialize for Duration {
	fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
		s.collect_str(self)
	}
}

impl<'de> de::Deserialize<'de> for Duration {
	fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		let raw = <std::borrow::Cow<'_, str>>::deserialize(d)?;
		raw.parse().map_err(de::Error::custom)
	}
}

#[derive(Debug, thiserror::Error)]
pub enum FromStrError {
	#[error(transparent)]
	Float(#[from] std::num::ParseFloatError),
	#[error(transparent)]
	OutOfRange(#[from] OutOfRange),
	#[error("unknown unit {0:?}")]
	UnknownUnit(String),
}

impl FromStr for Duration {
	type Err = FromStrError;

	fn from_str(raw: &str) -> Result<Self, Self::Err> {
		let amount_end = raw
			.bytes()
			.position(|ch| !ch.is_ascii_digit() && ch != b'-' && ch != b'.')
			.unwrap_or(raw.len());
		let (amount, unit) = raw.split_at(amount_end);
		let unit = unit.trim_start();

		let amount = amount.parse::<f32>()?;
		let scale = match unit.to_ascii_lowercase().as_str() {
			"us" | "µs" => 1.0,
			"ms" => 1_000.0,
			"s" => 1_000_000.0,
			_ => return Err(FromStrError::UnknownUnit(unit.to_owned())),
		};
		let micros = amount * scale;

		let micros = az::checked_cast(micros).ok_or(OutOfRange)?;
		Ok(Self { micros })
	}
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("value out of range")]
pub struct OutOfRange;

impl Duration {
	pub const MAX: Self = Self { micros: u32::MAX };

	pub fn new_secs_f32(secs: f32) -> Result<Self, OutOfRange> {
		az::checked_cast(secs * 1_000_000.0)
			.ok_or(OutOfRange)
			.map(Self::new_micros)
	}

	pub fn new_secs_f32_saturating(secs: f32) -> Self {
		Self::new_micros(az::saturating_cast::<_, u32>(secs * 1_000_000.0))
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
	pub fn advance(&mut self, elapsed: Duration) -> bool {
		self.micros = self.micros.saturating_sub(elapsed.micros);
		self.is_over()
	}

	/// Whether this duration has elapsed.
	pub fn is_over(self) -> bool {
		self.micros == 0
	}
}

impl From<Duration> for std::time::Duration {
	fn from(seconds: Duration) -> Self {
		Self::from_micros(seconds.micros.into())
	}
}

impl TryFrom<image::Delay> for Duration {
	type Error = OutOfRange;

	fn try_from(delay: image::Delay) -> Result<Self, OutOfRange> {
		let (numer, denom) = delay.numer_denom_ms();
		Self::new_millis_f32(az::cast::<_, f32>(numer) / az::cast::<_, f32>(denom))
	}
}

impl std::fmt::Display for Duration {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let micros: f32 = az::cast(self.micros);
		let (value, unit) = if micros < 1_000.0 {
			(micros, "us")
		} else if micros < 1_000_000.0 {
			(micros / 1_000.0, "ms")
		} else {
			(micros / 1_000_000.0, "s")
		};

		write!(formatter, "{value:.3} {unit}")
	}
}
