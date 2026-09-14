#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
//! In this file we define format-specific acknowledgements for richer inspection
//! of chart files, so we can get nice descriptions of charts, maybe what gamemode they are,
//! and what chart ID algorithms can be associated with it.
//!
//! This works by adding a new file extension to [`crate::format::RecognisedFormat`], and then
//! filling out all the `match` statements in this crate. There are a couple of dials you can
//! play with.

use backbeat_core::{
	BackbeatFile, ChartDesc, ChartFilename, ChartId, CustomIdAlgorithm, IdAlgorithm, Sha256,
};
use serde::Serialize;

mod extract;
mod format;
mod gamemode;
mod parsed;
mod types;
mod util;

use self::parsed::ParsedChart;

pub use self::{gamemode::RecognisedGamemode, types::ChartExtract};

/// Nab some metadata out of the bundle, and do some calculations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BundleInspection {
	pub chart_sha256: Sha256,
	pub description: ChartDesc,
	pub extract: ChartExtract,
	pub gamemode: RecognisedGamemode,
	pub chart_ids: Vec<ChartId>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum InspectError {
	#[error("We failed to parse this chart: {0}")]
	UnparsableChart(String),
}

/// Take a backbeat file and inspect it - get information about it.
///
/// This uses knowledge about games to get the best possible info. If we don't know about a
/// format, this will just say unknown chart.
///
/// We provide the following format specific metadata:
/// - "extra" chart ID algorithms, depending on what the format might want. These are algorithms that are "externally" useful,
///   i.e. other tools use them and it's nice for backbeat to be able to look charts up by them. At the moment, only md5 is
///   additionally strapped on, when the chart is a bms chart.
///
/// - song, chart, title info
/// - What "gamemode" this chart is for, if it can be figured out. This separates things like bms-5k from bms-7k and so on, when multiple different "gamemodes" are muxed into the same file extension.
pub fn inspect_bundle(bb: &BackbeatFile) -> Result<BundleInspection, InspectError> {
	let bytes = bb.chart.decompress();

	inspect(&bytes, &bb.filename)
}

/// Inspect a chart and get information about it. This is used by the packager to attach reasonable `desc`s
/// onto charts.
pub fn inspect(bytes: &[u8], filename: &ChartFilename) -> Result<BundleInspection, InspectError> {
	let chart_sha256 = Sha256::checksum_bytes(bytes);

	let parsed = match ParsedChart::parse(bytes, filename) {
		Ok(v) => v,
		Err(err) => {
			return Err(InspectError::UnparsableChart(err));
		}
	};

	let extract = extract(&parsed, filename);
	let description = describe(&extract);
	let chart_ids = filename
		.id_algorithms()
		.iter()
		.filter_map(|&algorithm| {
			algorithm.compute(bytes, filename).ok().map(|val| ChartId {
				alg: IdAlgorithm::Custom(
					CustomIdAlgorithm::new(&algorithm.to_string())
						.expect("built-in custom algorithm is valid"),
				),
				val,
			})
		})
		.collect();

	Ok(BundleInspection {
		chart_sha256,
		description,
		extract,
		gamemode: parsed.gamemode(),
		chart_ids,
	})
}

fn extract(chart: &ParsedChart, filename: &ChartFilename) -> ChartExtract {
	match chart {
		ParsedChart::Bms(chart) => extract::bms::extract(chart),
		ParsedChart::Bmson(chart) => extract::bmson::extract(chart),
		ParsedChart::Sm(chart) => extract::sm::extract(chart),
		ParsedChart::Ssc(chart) => extract::ssc::extract(chart),
		ParsedChart::Dwi(chart) => extract::dwi::extract(chart),
		ParsedChart::Ksh(chart) => extract::ksh::extract(chart, filename),
		ParsedChart::Kson(chart) => extract::kson::extract(chart),
	}
}

fn describe(extract: &ChartExtract) -> ChartDesc {
	let artist = extract
		.song
		.artist
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.unwrap_or("Unknown Artist");
	let title = extract
		.song
		.title
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.unwrap_or("Unknown Title");
	let credit = extract
		.chart
		.credit
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty());
	let chart_name = extract
		.chart
		.chart_name
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty());
	let suffix = match (credit, chart_name) {
		(Some(credit), Some(chart_name)) => format!(" ({credit}'s {chart_name})"),
		(Some(credit), None) => format!(" ({credit})"),
		(None, Some(chart_name)) => format!(" ({chart_name})"),
		(None, None) => String::new(),
	};

	ChartDesc::new_truncate(&format!("{artist} - {title}{suffix}"))
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{ChartData, ChartDesc, ChartFilename};

	use super::*;

	fn bundle(filename: &str, chart: &[u8]) -> BackbeatFile {
		BackbeatFile {
			filename: ChartFilename::from_path(filename).unwrap(),
			assets: HashMap::new(),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(chart).unwrap(),
		}
	}

	#[test]
	fn unknown_extension_is_error() {
		let inspection = inspect_bundle(&bundle("chart.googus", b"opaque"));
		assert!(inspection.is_err());
	}

	#[test]
	fn bms_bundle_is_supported() {
		let inspection = inspect_bundle(&bundle("chart.bms", b"#TITLE Test\n#BPM 120\n")).unwrap();
		assert_eq!(inspection.gamemode, RecognisedGamemode::Bms5k);
		assert_eq!(inspection.description.as_str(), "Unknown Artist - Test");
		assert_eq!(inspection.chart_ids.len(), 1);
	}
}

#[cfg(test)]
mod extractor_tests;
