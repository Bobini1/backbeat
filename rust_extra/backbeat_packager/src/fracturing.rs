//! Some chart files actually contain multiple charts.
//!
//! Fracturing is the process of breaking those into files that contain one chart each
//! Melding is the process of putting multiple charts back together.

pub(crate) mod dwi;
pub(crate) mod sm;
pub(crate) mod ssc;

use backbeat_core::{BackbeatFile, BackbeatFileError, ChartData};

use crate::format::Format;

/// Errors from fracturing a multi-chart source into single-chart blobs.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FractureError {
	#[error("chart contains no playable charts")]
	NoCharts,

	#[error("The file extension {0} cannot be fractured")]
	UnsupportedFormat(String),
}

/// Errors that can occur while combining fractured [`BackbeatFile`]s.
#[derive(Debug, thiserror::Error)]
pub enum MeldError {
	/// No input bundles were provided.
	#[error("at least one fractured bundle is required")]
	Empty,

	/// The bundle format does not support melding.
	#[error("The file extension {0} cannot be melded")]
	UnsupportedFormat(String),

	/// All bundles must retain the same source chart filename.
	#[error(
		"melding requires all bundles to have the same filename (bundle {input_index} differs)"
	)]
	NotSameFilename {
		/// The zero-based position of the incompatible bundle.
		input_index: usize,
	},

	/// All bundles must retain the same asset map.
	#[error("bundle {input_index} has a different asset map")]
	NotSameAssets {
		/// The zero-based position of the incompatible bundle.
		input_index: usize,
	},

	/// A bundle's compressed chart data could not be processed.
	#[error(transparent)]
	BackbeatFile(#[from] BackbeatFileError),

	/// The embedded SM charts could not be combined.
	#[error(transparent)]
	Sm(#[from] sm::MeldError),

	/// The embedded SSC charts could not be combined.
	#[error(transparent)]
	Ssc(#[from] ssc::MeldError),

	/// The embedded DWI charts could not be combined.
	#[error(transparent)]
	Dwi(#[from] dwi::MeldError),
}

/// Combine compatible fractured bundles into one uncompressed chart file.
pub fn meld_chart_bytes(bundles: &[BackbeatFile]) -> Result<Box<[u8]>, MeldError> {
	let Some(first) = bundles.first() else {
		return Err(MeldError::Empty);
	};
	let Some(format) = Format::from_path(first.filename.as_str()) else {
		return Err(MeldError::UnsupportedFormat(first.filename.to_string()));
	};
	if !format.can_meld() {
		return Err(MeldError::UnsupportedFormat(format.as_str().to_owned()));
	}

	for (input_index, bundle) in bundles.iter().enumerate().skip(1) {
		if bundle.filename != first.filename {
			return Err(MeldError::NotSameFilename { input_index });
		}
		if bundle.assets != first.assets {
			return Err(MeldError::NotSameAssets { input_index });
		}
	}

	let charts = bundles
		.iter()
		.map(|bundle| bundle.chart.decompress())
		.collect::<Vec<_>>();
	format.meld_bytes(&charts)
}

/// Combine compatible fractured bundles into a single bundle.
pub fn meld(bundles: &[BackbeatFile]) -> Result<BackbeatFile, MeldError> {
	let Some(first) = bundles.first() else {
		return Err(MeldError::Empty);
	};
	let chart = meld_chart_bytes(bundles)?;

	let mut melded = first.clone();
	melded.chart = ChartData::compress(&chart)?;
	Ok(melded)
}

#[cfg(test)]
mod tests {
	use backbeat_core::{ChartDesc, ChartFilename};

	use super::*;

	fn dwi_bundle(chart: &[u8]) -> BackbeatFile {
		BackbeatFile {
			filename: ChartFilename::from_path("song.dwi").unwrap(),
			assets: Default::default(),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(chart).unwrap(),
		}
	}

	#[test]
	fn melds_dwi_bundles() {
		let bundles = [
			dwi_bundle(b"#TITLE:Test;\n#SINGLE:EASY:3:22;\n"),
			dwi_bundle(b"#TITLE:Test;\n#SINGLE:HARD:9:44;\n"),
		];

		let melded = meld(&bundles).unwrap();
		assert_eq!(
			rg_formats::dwi::from_bytes(&melded.chart.decompress(), "song.dwi").len(),
			2
		);
	}
}
