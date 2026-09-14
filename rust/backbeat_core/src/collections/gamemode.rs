use std::{fmt, str::FromStr};

use serde_with::{DeserializeFromStr, SerializeDisplay};

/// A gamemode must at-least be a `[a-z0-9-]` non empty string.
///
/// We don't prescribe what gamemodes exist, just that they look like this format.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, SerializeDisplay, DeserializeFromStr,
)]
pub struct ValidGamemodeIdentifier(String);

impl ValidGamemodeIdentifier {
	pub fn new(value: &str) -> Result<Self, ValidGamemodeIdentifierError> {
		if value.is_empty() {
			return Err(ValidGamemodeIdentifierError::Empty);
		}
		if !value
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		{
			return Err(ValidGamemodeIdentifierError::Invalid(value.to_owned()));
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

impl fmt::Display for ValidGamemodeIdentifier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl FromStr for ValidGamemodeIdentifier {
	type Err = ValidGamemodeIdentifierError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		Self::new(value)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidGamemodeIdentifierError {
	#[error("gamemode identifier must not be empty")]
	Empty,
	#[error(
		"invalid gamemode identifier {0:?}; expected lowercase ASCII letters, digits, and hyphens"
	)]
	Invalid(String),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accepts_lowercase_ascii_letters_digits_and_hyphens() {
		for value in ["bms-7k", "sm-dance-single", "123", "a-b-c"] {
			assert_eq!(ValidGamemodeIdentifier::new(value).unwrap().as_str(), value);
		}
	}

	#[test]
	fn rejects_empty_and_invalid_characters() {
		assert_eq!(
			ValidGamemodeIdentifier::new(""),
			Err(ValidGamemodeIdentifierError::Empty)
		);
		for value in ["BMS-7K", "sm_dance_single", "sm dance single", "ü"] {
			assert!(matches!(
				ValidGamemodeIdentifier::new(value),
				Err(ValidGamemodeIdentifierError::Invalid(_))
			));
		}
	}

	#[test]
	fn serde_roundtrips_as_a_string() {
		let identifier = ValidGamemodeIdentifier::new("bms-7k").unwrap();
		let json = serde_json::to_string(&identifier).unwrap();
		assert_eq!(json, "\"bms-7k\"");
		assert_eq!(
			serde_json::from_str::<ValidGamemodeIdentifier>(&json).unwrap(),
			identifier
		);
	}
}
