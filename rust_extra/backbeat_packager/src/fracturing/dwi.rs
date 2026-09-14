//! Fracture and meld `.dwi` files.

use rg_formats::sm_msd::{self, MsdElement};
use sha2::{Digest, Sha256};

use super::FractureError;

/// Errors that can occur while melding fractured DWI files.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MeldError {
	/// A fracture must contain exactly one chart.
	#[error(
		"fractured DWI input {input_index} contains {chart_count} chart sections, expected one"
	)]
	InvalidFracture {
		/// The zero-based position of the incompatible input.
		input_index: usize,
		/// The number of chart sections in the input.
		chart_count: usize,
	},

	/// Fractures must have identical shared metadata.
	#[error("fractured DWI input {input_index} has incompatible shared metadata")]
	IncompatibleMetadata {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},

	/// The chart has no difficulty field.
	#[error("fractured DWI input {input_index} has a chart section without a difficulty")]
	MissingDifficulty {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},
}

pub(crate) fn fracture_bytes(bytes: &[u8]) -> Result<Vec<Box<[u8]>>, FractureError> {
	let msd = sm_msd::from_bytes(bytes);
	let header = msd
		.elements
		.iter()
		.filter(|element| !is_chart_tag(element))
		.cloned()
		.collect::<Vec<_>>();
	let charts = msd
		.elements
		.into_iter()
		.filter(is_chart_tag)
		.filter_map(|chart| {
			let mut elements = header.clone();
			elements.push(chart);
			let bytes = sm_msd::serialize_msd_elements(elements);
			(!rg_formats::dwi::from_bytes(&bytes, "chart.dwi").is_empty()).then_some(bytes)
		})
		.collect::<Vec<_>>();
	if charts.is_empty() {
		return Err(FractureError::NoCharts);
	}
	Ok(charts)
}

/// Recombine compatible, single-chart DWI files into one canonical DWI file.
pub(crate) fn meld_bytes(inputs: &[Vec<u8>]) -> Result<Box<[u8]>, MeldError> {
	let mut parent_id = None;
	let mut header = None;
	let mut charts = Vec::with_capacity(inputs.len());

	for (input_index, input) in inputs.iter().enumerate() {
		let msd = sm_msd::from_bytes(input);
		let chart_tags = msd
			.elements
			.iter()
			.filter(|element| is_chart_tag(element))
			.cloned()
			.collect::<Vec<_>>();
		if chart_tags.len() != 1 {
			return Err(MeldError::InvalidFracture {
				input_index,
				chart_count: chart_tags.len(),
			});
		}

		let shared_elements = msd
			.elements
			.iter()
			.filter(|element| !is_chart_tag(element))
			.cloned()
			.collect::<Vec<_>>();
		let shared_id = fracture_parent_id(&shared_elements);
		if let Some(expected_parent_id) = parent_id {
			if expected_parent_id != shared_id {
				return Err(MeldError::IncompatibleMetadata { input_index });
			}
		} else {
			parent_id = Some(shared_id);
			header = Some(shared_elements);
		}

		let chart = chart_tags
			.into_iter()
			.next()
			.expect("exactly one chart tag");
		let difficulty = chart_difficulty_key(&chart, input_index)?;
		let chart_id = Sha256::digest(sm_msd::serialize_msd_elements(vec![chart.clone()]));
		charts.push((difficulty, chart_id, chart));
	}

	charts.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

	let mut elements = header.unwrap_or_default();
	elements.extend(charts.into_iter().map(|(_, _, chart)| chart));
	Ok(sm_msd::serialize_msd_elements(elements))
}

fn is_chart_tag(element: &MsdElement) -> bool {
	matches!(
		element.tag.as_ref(),
		b"SINGLE" | b"DOUBLE" | b"COUPLE" | b"SOLO"
	)
}

fn fracture_parent_id(elements: &[MsdElement]) -> [u8; 32] {
	Sha256::digest(sm_msd::serialize_msd_elements(elements.to_vec())).into()
}

fn chart_difficulty_key(chart: &MsdElement, input_index: usize) -> Result<u8, MeldError> {
	let difficulty = chart
		.values
		.first()
		.ok_or(MeldError::MissingDifficulty { input_index })?;
	Ok(
		match difficulty.trim_ascii().to_ascii_lowercase().as_slice() {
			b"beginner" => 0,
			b"easy" | b"basic" | b"light" => 1,
			b"medium" | b"another" | b"trick" | b"standard" | b"difficult" => 2,
			b"hard" | b"ssr" | b"maniac" | b"heavy" => 3,
			b"smaniac" | b"challenge" | b"expert" | b"oni" => 4,
			b"edit" => 5,
			_ => 6,
		},
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fractures_each_playable_dwi_chart() {
		let bytes = b"#TITLE:Test;\n#SINGLE:BASIC:3:44;\n#DOUBLE:MANIAC:9:88;\n";
		let charts = fracture_bytes(bytes).unwrap();
		assert_eq!(charts.len(), 2);
		for chart in charts {
			assert_eq!(rg_formats::dwi::from_bytes(&chart, "chart.dwi").len(), 1);
		}
	}

	fn fractured_fixture() -> Vec<Vec<u8>> {
		[
			"#TITLE:Test;\n#FILE:song.ogg;\n#SINGLE:HARD:9:44;\n",
			"#TITLE:Test;\n#FILE:song.ogg;\n#SINGLE:EDIT:10:88;\n",
			"#TITLE:Test;\n#FILE:song.ogg;\n#SINGLE:EASY:3:22;\n",
		]
		.into_iter()
		.map(|bytes| bytes.as_bytes().to_vec())
		.collect()
	}

	#[test]
	fn meld_sorts_charts_by_difficulty() {
		let mut fractured = fractured_fixture();
		fractured.reverse();

		let melded = String::from_utf8(meld_bytes(&fractured).unwrap().into_vec()).unwrap();
		let easy = melded.find("#SINGLE:EASY").unwrap();
		let hard = melded.find("#SINGLE:HARD").unwrap();
		let edit = melded.find("#SINGLE:EDIT").unwrap();
		assert!(easy < hard && hard < edit);
		assert_eq!(
			rg_formats::dwi::from_bytes(melded.as_bytes(), "chart.dwi").len(),
			3
		);
	}

	#[test]
	fn meld_rejects_incompatible_shared_metadata() {
		let first = fractured_fixture()[0].clone();
		let second = b"#TITLE:Test;\n#FILE:other.ogg;\n#SINGLE:EASY:3:22;\n".to_vec();

		assert_eq!(
			meld_bytes(&[first, second]),
			Err(MeldError::IncompatibleMetadata { input_index: 1 })
		);
	}

	#[test]
	fn meld_requires_single_chart_inputs() {
		let input = b"#TITLE:Test;\n#SINGLE:EASY:3:22;\n#DOUBLE:HARD:9:44;\n".to_vec();
		assert_eq!(
			meld_bytes(&[input]),
			Err(MeldError::InvalidFracture {
				input_index: 0,
				chart_count: 2,
			})
		);
	}
}
