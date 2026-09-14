use std::fmt;
use std::str::FromStr;

use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::EnumIter;

/// How a chart is played. Every chart in backbeat has exactly one gamemode.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr, EnumIter,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum RecognisedGamemode {
	// BMS gamemodes
	#[cfg_attr(feature = "ts", ts(rename = "bms-5k"))]
	Bms5k,
	#[cfg_attr(feature = "ts", ts(rename = "bms-7k"))]
	Bms7k,
	#[cfg_attr(feature = "ts", ts(rename = "bms-10k"))]
	Bms10k,
	#[cfg_attr(feature = "ts", ts(rename = "bms-14k"))]
	Bms14k,

	// PMS gamemodes: 5 button (obscure), 9 button.
	#[cfg_attr(feature = "ts", ts(rename = "pms-5b"))]
	Pms5b,
	#[cfg_attr(feature = "ts", ts(rename = "pms-9b"))]
	Pms9b,

	// Key-Music-Source (24k/48k modes in bms)
	#[cfg_attr(feature = "ts", ts(rename = "kms-24k"))]
	Kms24k,
	#[cfg_attr(feature = "ts", ts(rename = "kms-48k"))]
	Kms48k,

	// Stepmania, on a dancepad
	/// 4 Panels on the floor in cardinal directions.
	#[cfg_attr(feature = "ts", ts(rename = "sm-dance-single"))]
	SmDanceSingle,
	/// 8 panels on the floor, the doubles variant of sm dance.
	#[cfg_attr(feature = "ts", ts(rename = "sm-dance-double"))]
	SmDanceDouble,
	/// 8 panels on the floor, played by two people
	#[cfg_attr(feature = "ts", ts(rename = "sm-dance-couple"))]
	SmDanceCouple,

	/// 6 panels on the floor; left-down-up-right with upright and upleft.
	#[cfg_attr(feature = "ts", ts(rename = "sm-dance-solo"))]
	SmDanceSolo,
	/// Six cabinet lighting channels, intended for ambient lighting rather than gameplay.
	#[cfg_attr(feature = "ts", ts(rename = "sm-lights-cabinet"))]
	SmLightsCabinet,

	/// 5 panels on the floor, arranged in an X formation.
	#[cfg_attr(feature = "ts", ts(rename = "sm-pump-single"))]
	SmPumpSingle,
	/// The middle 6 panels in a pump doubles setup.
	#[cfg_attr(feature = "ts", ts(rename = "sm-pump-halfdouble"))]
	SmPumpHalfdouble,
	/// 10 panels on the floor, the doubles variant of pump.
	#[cfg_attr(feature = "ts", ts(rename = "sm-pump-double"))]
	SmPumpDouble,
	/// 10 panels on the floor, played by two people.
	#[cfg_attr(feature = "ts", ts(rename = "sm-pump-couple"))]
	SmPumpCouple,
	/// A pump chart played as a coordinated routine by multiple players.
	#[cfg_attr(feature = "ts", ts(rename = "sm-pump-routine"))]
	SmPumpRoutine,

	/// A gamemode played with 6 keys and two analog knobs with a SDVX controller.
	#[cfg_attr(feature = "ts", ts(rename = "kshoot"))]
	Kshoot,

	/// This is an unknown gamemode. We don't know what game this is for, and haven't added
	/// support.
	#[cfg_attr(feature = "ts", ts(rename = "unknown"))]
	Unknown,
}

impl RecognisedGamemode {
	/// Whether this is a BMS-family gamemode.
	pub const fn is_bms(self) -> bool {
		matches!(
			self,
			Self::Bms5k
				| Self::Bms7k
				| Self::Bms10k
				| Self::Bms14k
				| Self::Pms5b
				| Self::Pms9b
				| Self::Kms24k
				| Self::Kms48k
		)
	}

	/// Whether this is a StepMania-family gamemode.
	pub const fn is_stepmania(self) -> bool {
		matches!(
			self,
			Self::SmDanceSingle
				| Self::SmDanceDouble
				| Self::SmDanceCouple
				| Self::SmDanceSolo
				| Self::SmLightsCabinet
				| Self::SmPumpSingle
				| Self::SmPumpHalfdouble
				| Self::SmPumpDouble
				| Self::SmPumpCouple
				| Self::SmPumpRoutine
		)
	}

	/// Stable wire/DB name for this gamemode (e.g. `"bms-14k"`).
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Bms5k => "bms-5k",
			Self::Bms7k => "bms-7k",
			Self::Bms10k => "bms-10k",
			Self::Bms14k => "bms-14k",
			Self::Pms5b => "pms-5b",
			Self::Pms9b => "pms-9b",
			Self::Kms24k => "kms-24k",
			Self::Kms48k => "kms-48k",
			Self::SmDanceSingle => "sm-dance-single",
			Self::SmDanceDouble => "sm-dance-double",
			Self::SmDanceCouple => "sm-dance-couple",
			Self::SmDanceSolo => "sm-dance-solo",
			Self::SmLightsCabinet => "sm-lights-cabinet",
			Self::SmPumpSingle => "sm-pump-single",
			Self::SmPumpHalfdouble => "sm-pump-halfdouble",
			Self::SmPumpDouble => "sm-pump-double",
			Self::SmPumpCouple => "sm-pump-couple",
			Self::SmPumpRoutine => "sm-pump-routine",
			Self::Kshoot => "kshoot",
			Self::Unknown => "unknown",
		}
	}
}

impl fmt::Display for RecognisedGamemode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for RecognisedGamemode {
	type Err = ParseGamemodeError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"bms-5k" => Self::Bms5k,
			"bms-7k" => Self::Bms7k,
			"bms-10k" => Self::Bms10k,
			"bms-14k" => Self::Bms14k,
			"pms-5b" => Self::Pms5b,
			"pms-9b" => Self::Pms9b,
			"kms-24k" => Self::Kms24k,
			"kms-48k" => Self::Kms48k,
			"sm-dance-single" => Self::SmDanceSingle,
			"sm-dance-double" => Self::SmDanceDouble,
			"sm-dance-couple" => Self::SmDanceCouple,
			"sm-dance-solo" => Self::SmDanceSolo,
			"sm-lights-cabinet" => Self::SmLightsCabinet,
			"sm-pump-single" => Self::SmPumpSingle,
			"sm-pump-halfdouble" => Self::SmPumpHalfdouble,
			"sm-pump-double" => Self::SmPumpDouble,
			"sm-pump-couple" => Self::SmPumpCouple,
			"sm-pump-routine" => Self::SmPumpRoutine,
			"kshoot" => Self::Kshoot,
			"unknown" => Self::Unknown,
			_ => return Err(ParseGamemodeError(s.to_owned())),
		})
	}
}

/// Error returned when an unrecognised gamemode string is encountered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unrecognised gamemode: {0:?}")]
pub struct ParseGamemodeError(String);

#[cfg(feature = "sqlx")]
mod sqlx {
	use super::RecognisedGamemode;
	use sqlx::decode::Decode;
	use sqlx::encode::{Encode, IsNull};
	use sqlx::error::BoxDynError;
	use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
	use sqlx::{Database, Sqlite, Type};

	impl Type<Sqlite> for RecognisedGamemode {
		fn type_info() -> SqliteTypeInfo {
			<&str as Type<Sqlite>>::type_info()
		}

		fn compatible(ty: &SqliteTypeInfo) -> bool {
			<&str as Type<Sqlite>>::compatible(ty)
		}
	}

	impl<'q> Encode<'q, Sqlite> for RecognisedGamemode {
		fn encode_by_ref(
			&self,
			buf: &mut <Sqlite as Database>::ArgumentBuffer,
		) -> Result<IsNull, BoxDynError> {
			let text = self.to_string();
			<String as Encode<'q, Sqlite>>::encode_by_ref(&text, buf)
		}
	}

	impl<'r> Decode<'r, Sqlite> for RecognisedGamemode {
		fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
			let value: String = Decode::<Sqlite>::decode(value)?;
			value
				.parse::<Self>()
				.map_err(|error| Box::new(error) as BoxDynError)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use strum::IntoEnumIterator as _;

	#[test]
	fn display_and_from_str_roundtrip() {
		for mode in RecognisedGamemode::iter() {
			let s = mode.to_string();
			assert_eq!(RecognisedGamemode::from_str(&s).unwrap(), mode, "{s}");
		}
	}

	#[test]
	fn wire_names() {
		let cases = [
			(RecognisedGamemode::Bms5k, "bms-5k"),
			(RecognisedGamemode::Bms7k, "bms-7k"),
			(RecognisedGamemode::Bms10k, "bms-10k"),
			(RecognisedGamemode::Bms14k, "bms-14k"),
			(RecognisedGamemode::Pms5b, "pms-5b"),
			(RecognisedGamemode::Pms9b, "pms-9b"),
			(RecognisedGamemode::Kms24k, "kms-24k"),
			(RecognisedGamemode::Kms48k, "kms-48k"),
			(RecognisedGamemode::SmDanceSingle, "sm-dance-single"),
			(RecognisedGamemode::SmDanceDouble, "sm-dance-double"),
			(RecognisedGamemode::SmDanceCouple, "sm-dance-couple"),
			(RecognisedGamemode::SmDanceSolo, "sm-dance-solo"),
			(RecognisedGamemode::SmLightsCabinet, "sm-lights-cabinet"),
			(RecognisedGamemode::SmPumpSingle, "sm-pump-single"),
			(RecognisedGamemode::SmPumpHalfdouble, "sm-pump-halfdouble"),
			(RecognisedGamemode::SmPumpDouble, "sm-pump-double"),
			(RecognisedGamemode::SmPumpCouple, "sm-pump-couple"),
			(RecognisedGamemode::SmPumpRoutine, "sm-pump-routine"),
			(RecognisedGamemode::Kshoot, "kshoot"),
			(RecognisedGamemode::Unknown, "unknown"),
		];
		assert_eq!(cases.len(), RecognisedGamemode::iter().count());
		for (mode, wire) in cases {
			assert_eq!(mode.to_string(), wire);
			assert_eq!(RecognisedGamemode::from_str(wire).unwrap(), mode);
			assert_eq!(serde_json::to_string(&mode).unwrap(), format!("\"{wire}\""));
		}
	}

	#[test]
	fn bms_family_modes_are_recognised() {
		assert!(RecognisedGamemode::Bms5k.is_bms());
		assert!(RecognisedGamemode::Pms9b.is_bms());
		assert!(RecognisedGamemode::Kms48k.is_bms());
		assert!(!RecognisedGamemode::SmDanceSingle.is_bms());
		assert!(!RecognisedGamemode::Kshoot.is_bms());
	}

	#[test]
	fn stepmania_family_modes_are_recognised() {
		assert!(RecognisedGamemode::SmDanceSingle.is_stepmania());
		assert!(RecognisedGamemode::SmDanceCouple.is_stepmania());
		assert!(RecognisedGamemode::SmLightsCabinet.is_stepmania());
		assert!(RecognisedGamemode::SmPumpDouble.is_stepmania());
		assert!(RecognisedGamemode::SmPumpCouple.is_stepmania());
		assert!(RecognisedGamemode::SmPumpRoutine.is_stepmania());
		assert!(!RecognisedGamemode::Bms7k.is_stepmania());
		assert!(!RecognisedGamemode::Kshoot.is_stepmania());
	}
}
