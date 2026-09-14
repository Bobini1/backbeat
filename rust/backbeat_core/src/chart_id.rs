mod md5;

use std::{fmt, str::FromStr};

use derive_more::Display;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::ChartFilename;

/// A chart ID is a combination of the algorithm used to calculate it and its value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct ChartId {
	pub alg: IdAlgorithm,
	pub val: String,
}

impl fmt::Display for ChartId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}/{}", self.alg, self.val)
	}
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum ParseChartIdError {
	#[error("expected a chart ID in `algorithm/value` form")]
	MissingSeparator,
	#[error("invalid chart ID algorithm: {0}")]
	InvalidAlgorithm(#[source] IdAlgorithmParseError),
	#[error("The value for this string contains invalid characters such as '/', '?' and '#'")]
	InvalidValue,
}

impl FromStr for ChartId {
	type Err = ParseChartIdError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (alg, val) = s
			.split_once('/')
			.ok_or(ParseChartIdError::MissingSeparator)?;
		let alg = IdAlgorithm::from_str(alg).map_err(ParseChartIdError::InvalidAlgorithm)?;
		if val.contains(['/', '?', '#']) {
			return Err(ParseChartIdError::InvalidValue);
		}
		Ok(Self {
			alg,
			val: val.to_owned(),
		})
	}
}

/// A wrapper for chart identifier algorithms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub enum IdAlgorithm {
	Sha256,
	Custom(CustomIdAlgorithm),
}

impl fmt::Display for IdAlgorithm {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Sha256 => f.write_str("sha256"),
			Self::Custom(algorithm) => algorithm.fmt(f),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(transparent)]
pub struct IdAlgorithmParseError(#[from] CustomIdAlgorithmError);

impl FromStr for IdAlgorithm {
	type Err = IdAlgorithmParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s == "sha256" {
			return Ok(Self::Sha256);
		}
		CustomIdAlgorithm::from_str(s)
			.map(Self::Custom)
			.map_err(IdAlgorithmParseError::from)
	}
}

/// Custom chart id algorithms must be lowercase, numbers and use only hyphens.
///
/// We "acknowledge" any valid string, but we only currently
/// recognise a couple of them.
///
/// They can't start or end with hyphens and can't have two hyphens in a row.
///
/// They can't be longer than 128 characters.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, SerializeDisplay, DeserializeFromStr,
)]
pub struct CustomIdAlgorithm(String);

impl CustomIdAlgorithm {
	pub const MAX_LENGTH: usize = 128;

	pub fn new(value: &str) -> Result<Self, CustomIdAlgorithmError> {
		if value.is_empty() {
			return Err(CustomIdAlgorithmError::Empty);
		}
		if value == "sha256" {
			return Err(CustomIdAlgorithmError::Reserved);
		}
		if value.len() > Self::MAX_LENGTH {
			return Err(CustomIdAlgorithmError::TooLong);
		}

		if value.split('-').any(|segment| {
			segment.is_empty()
				|| !segment
					.bytes()
					.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
		}) {
			return Err(CustomIdAlgorithmError::Invalid(value.to_owned()));
		}
		Ok(Self(value.to_owned()))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn into_string(self) -> String {
		self.0
	}
}

impl fmt::Display for CustomIdAlgorithm {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl FromStr for CustomIdAlgorithm {
	type Err = CustomIdAlgorithmError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		Self::new(value)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustomIdAlgorithmError {
	#[error("custom ID algorithm must not be empty")]
	Empty,
	#[error("sha256 is reserved for the built-in ID algorithm")]
	Reserved,
	#[error(
		"invalid custom ID algorithm {0:?}; expected lowercase ASCII letters and digits separated by single hyphens"
	)]
	Invalid(String),
	#[error("chart ID algorithm names must not be longer than 128 characters")]
	TooLong,
}

/// A non-SHA-256 chart identifier algorithm.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, Display, SerializeDisplay, DeserializeFromStr,
)]
#[display(rename_all = "kebab-case")]
pub enum RecognisedChartIdAlgorithm {
	#[display("md5")]
	Md5,
}

impl FromStr for RecognisedChartIdAlgorithm {
	type Err = derive_more::FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"md5" => Ok(Self::Md5),
			_ => Err(derive_more::FromStrError::new(std::any::type_name::<Self>())),
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdAlgorithmError {
	#[error("This file could not be parsed")]
	#[allow(unused)]
	UnprocessableFile,
}

impl RecognisedChartIdAlgorithm {
	/// Calculate this additional chartID algorithm.
	#[allow(clippy::unnecessary_wraps)]
	pub fn compute(
		self,
		chart_bytes: &[u8],
		_path: &ChartFilename,
	) -> Result<String, IdAlgorithmError> {
		match self {
			Self::Md5 => Ok(md5::compute(chart_bytes)),
		}
	}

	/// Turn this into the more generic "CustomIdAlgorithm"
	pub fn into_custom(self) -> CustomIdAlgorithm {
		CustomIdAlgorithm::new(match self {
			Self::Md5 => "md5",
		})
		.expect("must be valid")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accepts_structured_custom_algorithms() {
		for value in ["md5", "groovestats-v3", "abc123", "a-b-c"] {
			assert_eq!(CustomIdAlgorithm::new(value).unwrap().as_str(), value);
		}
	}

	#[test]
	fn rejects_invalid_custom_algorithms() {
		assert_eq!(
			CustomIdAlgorithm::new(""),
			Err(CustomIdAlgorithmError::Empty)
		);
		assert_eq!(
			CustomIdAlgorithm::new("sha256"),
			Err(CustomIdAlgorithmError::Reserved)
		);
		for value in [
			"SHA256",
			"custom_algorithm",
			"custom algorithm",
			"custom-ü",
			"-custom",
			"custom-",
			"custom--algorithm",
		] {
			assert!(matches!(
				CustomIdAlgorithm::new(value),
				Err(CustomIdAlgorithmError::Invalid(_))
			));
		}
	}

	#[test]
	fn rejects_custom_algorithms_over_max_length() {
		let max_length = "a".repeat(CustomIdAlgorithm::MAX_LENGTH);
		assert!(CustomIdAlgorithm::new(&max_length).is_ok());

		let too_long = "a".repeat(CustomIdAlgorithm::MAX_LENGTH + 1);
		assert_eq!(
			CustomIdAlgorithm::new(&too_long),
			Err(CustomIdAlgorithmError::TooLong)
		);
	}

	#[test]
	fn parses_and_displays_id_algorithms() {
		assert_eq!(
			IdAlgorithm::from_str("sha256").unwrap(),
			IdAlgorithm::Sha256
		);
		assert!(IdAlgorithm::from_str("SHA256").is_err());

		let algorithm = IdAlgorithm::from_str("future-algorithm").unwrap();
		assert_eq!(algorithm.to_string(), "future-algorithm");
		assert_eq!(
			algorithm,
			IdAlgorithm::Custom(CustomIdAlgorithm::new("future-algorithm").unwrap())
		);
	}

	#[test]
	fn chart_id_serde_accepts_unknown_valid_algorithm() {
		let json = "\"future-algorithm/abc123\"";
		let chart_id: ChartId = serde_json::from_str(json).unwrap();
		assert_eq!(chart_id.to_string(), "future-algorithm/abc123");
		assert_eq!(serde_json::to_string(&chart_id).unwrap(), json);
	}

	#[test]
	fn chart_id_rejects_invalid_algorithm() {
		assert!(matches!(
			ChartId::from_str("future--algorithm/abc123"),
			Err(ParseChartIdError::InvalidAlgorithm(_))
		));
	}
}
