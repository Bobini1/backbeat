//! A type so users can write "8Mi" instead of 8388608.
//!
//! I stole this from k8s.
//!
//! Quantities are written as strings like `"16Ki"`, `"8Mi"`, or `"10k"`.
//! Binary suffixes (`Ki`, `Mi`, `Gi`, `Ti`) use powers of 1024; decimal
//! suffixes (`k`, `M`, `G`, `T`) use powers of 1000.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A non-negative byte count parsed from a k8s-style quantity string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export), ts(type = "string"))]
pub struct ByteSize(pub u64);

impl ByteSize {
	pub const ZERO: Self = Self(0);

	/// Parse a k8s-style byte quantity string.
	pub fn parse(s: &str) -> Result<Self, ParseByteSizeError> {
		let s = s.trim();
		if s.is_empty() {
			return Err(ParseByteSizeError::Empty);
		}

		let split = s
			.char_indices()
			.find(|(_, c)| !c.is_ascii_digit())
			.map(|(idx, _)| idx)
			.unwrap_or(s.len());

		let (number, suffix) = s.split_at(split);
		if number.is_empty() {
			return Err(ParseByteSizeError::Invalid {
				input: s.to_owned(),
			});
		}

		let value: u64 = number.parse().map_err(|_| ParseByteSizeError::Invalid {
			input: s.to_owned(),
		})?;

		let multiplier =
			suffix_multiplier(suffix).ok_or_else(|| ParseByteSizeError::UnknownSuffix {
				suffix: suffix.to_owned(),
			})?;

		let bytes = value
			.checked_mul(multiplier)
			.ok_or(ParseByteSizeError::Overflow)?;

		Ok(Self(bytes))
	}

	/// Convert to `usize`, erroring if the value exceeds the platform limit.
	pub fn as_usize(self) -> Result<usize, ParseByteSizeError> {
		usize::try_from(self.0).map_err(|_| ParseByteSizeError::Overflow)
	}
}

fn suffix_multiplier(suffix: &str) -> Option<u64> {
	match suffix.to_ascii_lowercase().as_str() {
		"" => Some(1),
		"ki" => Some(1024),
		"mi" => Some(1024_u64.pow(2)),
		"gi" => Some(1024_u64.pow(3)),
		"ti" => Some(1024_u64.pow(4)),
		"k" => Some(1_000),
		"m" => Some(1_000_u64.pow(2)),
		"g" => Some(1_000_u64.pow(3)),
		"t" => Some(1_000_u64.pow(4)),
		_ => None,
	}
}

impl fmt::Display for ByteSize {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		const BINARY: [(&str, u64); 4] = [
			("Ti", 1024_u64.pow(4)),
			("Gi", 1024_u64.pow(3)),
			("Mi", 1024_u64.pow(2)),
			("Ki", 1024),
		];

		if self.0 == 0 {
			return f.write_str("0");
		}

		for (suffix, unit) in BINARY {
			if self.0.is_multiple_of(unit) {
				return write!(f, "{}{suffix}", self.0 / unit);
			}
		}

		write!(f, "{}", self.0)
	}
}

impl Serialize for ByteSize {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> Deserialize<'de> for ByteSize {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;

		impl serde::de::Visitor<'_> for Visitor {
			type Value = ByteSize;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				formatter.write_str("a k8s-style byte quantity string (e.g. \"16Ki\", \"10k\")")
			}

			fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
				ByteSize::parse(value).map_err(E::custom)
			}

			fn visit_i64<E: serde::de::Error>(self, _value: i64) -> Result<Self::Value, E> {
				Err(E::custom(
					"byte quantities must be strings (e.g. \"16Ki\"), not integers",
				))
			}

			fn visit_u64<E: serde::de::Error>(self, _value: u64) -> Result<Self::Value, E> {
				Err(E::custom(
					"byte quantities must be strings (e.g. \"16Ki\"), not integers",
				))
			}
		}

		deserializer.deserialize_any(Visitor)
	}
}

/// Errors from parsing a byte quantity string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseByteSizeError {
	#[error("byte quantity must not be empty")]
	Empty,

	#[error("invalid byte quantity {input:?}")]
	Invalid { input: String },

	#[error("unknown byte quantity suffix {suffix:?}")]
	UnknownSuffix { suffix: String },

	#[error("byte quantity overflow")]
	Overflow,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_binary_suffixes() {
		assert_eq!(ByteSize::parse("16Ki").unwrap(), ByteSize(16 * 1024));
		assert_eq!(ByteSize::parse("32Mi").unwrap(), ByteSize(32 * 1024 * 1024));
		assert_eq!(ByteSize::parse("1Gi").unwrap(), ByteSize(1024_u64.pow(3)));
		assert_eq!(
			ByteSize::parse("2Ti").unwrap(),
			ByteSize(2 * 1024_u64.pow(4))
		);
	}

	#[test]
	fn parses_decimal_suffixes() {
		assert_eq!(ByteSize::parse("10k").unwrap(), ByteSize(10_000));
		assert_eq!(ByteSize::parse("1M").unwrap(), ByteSize(1_000_000));
		assert_eq!(ByteSize::parse("2G").unwrap(), ByteSize(2_000_000_000));
		assert_eq!(ByteSize::parse("1T").unwrap(), ByteSize(1_000_000_000_000));
	}

	#[test]
	fn parses_raw_bytes() {
		assert_eq!(ByteSize::parse("0").unwrap(), ByteSize::ZERO);
		assert_eq!(ByteSize::parse("16384").unwrap(), ByteSize(16_384));
	}

	#[test]
	fn trims_whitespace() {
		assert_eq!(ByteSize::parse("  16Ki  ").unwrap(), ByteSize(16 * 1024));
	}

	#[test]
	fn rejects_invalid_input() {
		assert_eq!(ByteSize::parse(""), Err(ParseByteSizeError::Empty));
		assert_eq!(
			ByteSize::parse("Ki"),
			Err(ParseByteSizeError::Invalid {
				input: "Ki".to_owned()
			})
		);
		assert_eq!(
			ByteSize::parse("16xz"),
			Err(ParseByteSizeError::UnknownSuffix {
				suffix: "xz".to_owned()
			})
		);
	}

	#[test]
	fn parses_case_insensitive_suffixes() {
		assert_eq!(ByteSize::parse("16ki").unwrap(), ByteSize(16 * 1024));
		assert_eq!(ByteSize::parse("16KI").unwrap(), ByteSize(16 * 1024));
		assert_eq!(ByteSize::parse("32MI").unwrap(), ByteSize(32 * 1024 * 1024));
		assert_eq!(ByteSize::parse("10K").unwrap(), ByteSize(10_000));
		assert_eq!(ByteSize::parse("16m").unwrap(), ByteSize(16_000_000));
	}

	#[test]
	fn rejects_overflow() {
		assert!(ByteSize::parse(&format!("{}Ti", u64::MAX)).is_err());
	}

	#[test]
	fn display_uses_binary_suffixes() {
		assert_eq!(ByteSize(0).to_string(), "0");
		assert_eq!(ByteSize(16 * 1024).to_string(), "16Ki");
		assert_eq!(ByteSize(32 * 1024 * 1024).to_string(), "32Mi");
		assert_eq!(ByteSize(10_000).to_string(), "10000");
	}

	#[test]
	fn serde_roundtrip() {
		let value = ByteSize(16 * 1024);
		let json = serde_json::to_string(&value).unwrap();
		assert_eq!(json, "\"16Ki\"");
		let parsed: ByteSize = serde_json::from_str(&json).unwrap();
		assert_eq!(parsed, value);
	}

	#[test]
	fn serde_rejects_integer() {
		let err = serde_json::from_str::<ByteSize>("16384").unwrap_err();
		assert!(err.to_string().contains("must be strings"));
	}
}
