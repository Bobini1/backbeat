//! Parsed chart content and derived gamemode views.

use backbeat_core::ChartFilename;

use crate::RecognisedGamemode;
use crate::format::RecognisedFormat;

/// A fully parsed chart for one of the formats backbeat recognises.
#[derive(Debug, Clone)]
pub(crate) enum ParsedChart {
	Bms(Box<rg_formats::bms::Chart>),
	Bmson(Box<rg_formats::bmson::Bmson>),
	Sm(Box<rg_formats::sm::Chart>),
	Ssc(Box<rg_formats::ssc::SscFile>),
	Dwi(Box<rg_formats::sm::Chart>),
	Ksh(Box<rg_formats::ksh::Chart>),
	Kson(Box<rg_formats::kson::Kson>),
}

impl ParsedChart {
	/// Parse chart bytes for the given format.
	///
	/// `path` is used by SM/SSC parsers for path-relative metadata; a filename
	/// stub is fine when the chart is not on disk.
	///
	/// For formats that package one chart per file (`.sm` / `.ssc`), the bytes
	/// must contain exactly one playable chart — zero or multiple is `None`.
	pub(crate) fn parse(bytes: &[u8], fname: &ChartFilename) -> Result<Self, String> {
		Ok(
			match RecognisedFormat::from_filename(fname.as_str())
				.ok_or_else(|| "Unrecognised file extension".to_string())?
			{
				ext @ (RecognisedFormat::Bms
				| RecognisedFormat::Bme
				| RecognisedFormat::Bml
				| RecognisedFormat::Pms) => Self::Bms(Box::new(
					rg_formats::bms::from_bytes(
						bytes,
						ext.as_str(),
						rg_formats::bms::BmsRandomStrategy::AlwaysFirstBranch,
					)
					.map_err(|e| e.to_string())?,
				)),
				RecognisedFormat::Bmson => Self::Bmson(Box::new(
					rg_formats::bmson::from_bytes(bytes).map_err(|e| e.to_string())?,
				)),
				RecognisedFormat::Sm => {
					let mut charts =
						rg_formats::sm::from_bytes(bytes, fname).map_err(|e| e.to_string())?;
					if charts.len() != 1 {
						return Err("No charts in chart file!".to_string());
					}
					Self::Sm(Box::new(charts.pop().expect("asserted right above")))
				}
				RecognisedFormat::Ssc => {
					let file = rg_formats::ssc::from_bytes(bytes, fname);
					if file.charts.len() != 1 {
						return Err("No charts in chart file!".to_string());
					}
					Self::Ssc(Box::new(file))
				}
				RecognisedFormat::Dwi => {
					let mut charts = rg_formats::dwi::from_bytes(bytes, fname);
					if charts.len() != 1 {
						return Err("No charts in chart file!".to_string());
					}
					Self::Dwi(Box::new(charts.pop().expect("asserted right above")))
				}
				RecognisedFormat::Ksh => Self::Ksh(Box::new(
					rg_formats::ksh::from_bytes(bytes).map_err(|e| e.to_string())?,
				)),
				RecognisedFormat::Kson => Self::Kson(Box::new(
					rg_formats::kson::from_bytes(bytes).map_err(|e| e.to_string())?,
				)),
			},
		)
	}

	/// The gamemode for this chart. The special gamemode [`Gamemode::Unknown`] is returned
	/// if we don't yet know what gamemode this chart is for. This allows backbeat to still
	/// provide decent generalised support for unrecognised gamemodes/files.
	pub(crate) fn gamemode(&self) -> RecognisedGamemode {
		crate::gamemode::infer(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::RecognisedGamemode;

	fn filename(value: &str) -> ChartFilename {
		ChartFilename::from_path(value).unwrap()
	}

	#[test]
	fn sm_missing_fields_errors() {
		assert!(ParsedChart::parse(b"", &filename("chart.sm")).is_err());
	}

	#[test]
	fn sm_gamemode_dance_single() {
		let bytes = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0.000=120.000;\n#NOTES:\n     dance-single:\n     :\n     Hard:\n     8:\n     0.000,0.000,0.000,0.000,0.000:\n0000\n,\n0000\n;\n";
		let mode = ParsedChart::parse(bytes, &filename("chart.sm"))
			.unwrap()
			.gamemode();
		assert_eq!(mode, RecognisedGamemode::SmDanceSingle);
	}

	#[test]
	fn sm_rejects_multiple_charts() {
		let bytes = b"#TITLE:T;\n#BPMS:0=120;\n\
			#NOTES:dance-single:A:Easy:1:0:0000;\n\
			#NOTES:dance-single:B:Hard:9:0:0000;\n";
		assert!(ParsedChart::parse(bytes, &filename("chart.sm")).is_err());
	}

	#[test]
	fn ssc_rejects_multiple_charts() {
		let bytes = b"#TITLE:T;\n\
			#NOTEDATA:;\n#STEPSTYPE:dance-single;\n#DIFFICULTY:Easy;\n#METER:1;\n#NOTES:\n0000\n;\n\
			#NOTEDATA:;\n#STEPSTYPE:dance-single;\n#DIFFICULTY:Hard;\n#METER:9;\n#NOTES:\n0000\n;\n";
		assert!(ParsedChart::parse(bytes, &filename("chart.ssc")).is_err());
	}
}
