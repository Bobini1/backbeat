//! Fracture and meld `.ssc` files.

use rg_formats::sm_msd::{self, MsdElement};
use sha2::{Digest, Sha256};

use super::FractureError;

/// Errors that can occur while melding fractured SSC files.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MeldError {
	/// A fracture must contain exactly one chart.
	#[error(
		"fractured SSC input {input_index} contains {notedata_count} #NOTEDATA sections, expected one"
	)]
	InvalidFracture {
		/// The zero-based position of the incompatible input.
		input_index: usize,
		/// The number of chart sections in the input.
		notedata_count: usize,
	},

	/// Fractures must have identical shared metadata.
	#[error("fractured SSC input {input_index} has incompatible shared metadata")]
	IncompatibleMetadata {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},

	/// The chart has no difficulty field.
	#[error("fractured SSC input {input_index} has a #NOTEDATA section without a difficulty")]
	MissingDifficulty {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},
}

/// Split a multi-chart SSC into one SSC blob per `#NOTEDATA` block.
pub(crate) fn fracture_bytes(bytes: &[u8]) -> Result<Vec<Box<[u8]>>, FractureError> {
	let msd = sm_msd::from_bytes(bytes);
	let split_pos = msd
		.elements
		.iter()
		.position(|el| &*el.tag == b"NOTEDATA")
		.unwrap_or(msd.elements.len());

	let (header, chart_stream) = msd.elements.split_at(split_pos);
	let header = header.to_vec();

	let mut chart_groups: Vec<Vec<MsdElement>> = Vec::new();
	let mut current: Vec<MsdElement> = Vec::new();
	for el in chart_stream {
		if &*el.tag == b"NOTEDATA" && !current.is_empty() {
			chart_groups.push(std::mem::take(&mut current));
		}
		current.push(el.clone());
	}
	if !current.is_empty() {
		chart_groups.push(current);
	}

	if chart_groups.is_empty() {
		return Err(FractureError::NoCharts);
	}

	Ok(chart_groups
		.into_iter()
		.map(|group| {
			let mut elements = header.clone();
			elements.extend(group);
			sm_msd::serialize_msd_elements(elements)
		})
		.collect())
}

/// Recombine compatible, single-chart SSC files into one canonical SSC file.
pub(crate) fn meld_bytes(inputs: &[Vec<u8>]) -> Result<Box<[u8]>, MeldError> {
	let mut parent_id = None;
	let mut header = None;
	let mut charts = Vec::with_capacity(inputs.len());

	for (input_index, input) in inputs.iter().enumerate() {
		let msd = sm_msd::from_bytes(input);
		let split_pos = msd
			.elements
			.iter()
			.position(|el| &*el.tag == b"NOTEDATA")
			.unwrap_or(msd.elements.len());
		let (shared_elements, chart_stream) = msd.elements.split_at(split_pos);

		let notedata_count = chart_stream
			.iter()
			.filter(|el| &*el.tag == b"NOTEDATA")
			.count();
		if notedata_count != 1 {
			return Err(MeldError::InvalidFracture {
				input_index,
				notedata_count,
			});
		}

		let shared_id = fracture_parent_id(shared_elements);
		if let Some(expected_parent_id) = parent_id {
			if expected_parent_id != shared_id {
				return Err(MeldError::IncompatibleMetadata { input_index });
			}
		} else {
			parent_id = Some(shared_id);
			header = Some(shared_elements.to_vec());
		}

		let difficulty = chart_difficulty_key(chart_stream, input_index)?;
		let chart_id = Sha256::digest(sm_msd::serialize_msd_elements(chart_stream.to_vec()));
		charts.push((difficulty, chart_id, chart_stream.to_vec()));
	}

	charts.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

	let mut elements = header.unwrap_or_default();
	elements.extend(charts.into_iter().flat_map(|(_, _, chart)| chart));
	Ok(sm_msd::serialize_msd_elements(elements))
}

fn fracture_parent_id(elements: &[MsdElement]) -> [u8; 32] {
	Sha256::digest(sm_msd::serialize_msd_elements(elements.to_vec())).into()
}

fn chart_difficulty_key(
	chart_stream: &[MsdElement],
	input_index: usize,
) -> Result<(u8, Box<[u8]>), MeldError> {
	let difficulty = chart_stream
		.iter()
		.find(|el| &*el.tag == b"DIFFICULTY")
		.and_then(|el| el.values.first())
		.ok_or(MeldError::MissingDifficulty { input_index })?;

	let rank = match difficulty.to_ascii_lowercase().as_slice() {
		b"beginner" => 0,
		b"easy" | b"basic" | b"light" => 1,
		b"medium" | b"another" | b"trick" | b"standard" | b"difficult" => 2,
		b"hard" | b"ssr" | b"maniac" | b"heavy" => 3,
		b"challenge" | b"expert" | b"oni" => 4,
		b"edit" => 5,
		_ => 6,
	};
	let edit_name = if rank == 5 {
		chart_stream
			.iter()
			.find(|el| &*el.tag == b"DESCRIPTION" || &*el.tag == b"CHARTNAME")
			.and_then(|el| el.values.first())
			.cloned()
			.unwrap_or_default()
	} else {
		Box::default()
	};

	Ok((rank, edit_name))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_ssc(charts: &[(&str, &str, &str, usize, &str)]) -> Vec<u8> {
		let mut s = String::from(
			"#TITLE:Test Song;\n\
			 #ARTIST:Test Artist;\n\
			 #MUSIC:song.ogg;\n\
			 #BPMS:0.000=120.000;\n",
		);
		for (steps_type, description, diff, level, notes) in charts {
			s.push_str(&format!(
				"#NOTEDATA:;\n\
				 #STEPSTYPE:{steps_type};\n\
				 #DESCRIPTION:{description};\n\
				 #DIFFICULTY:{diff};\n\
				 #METER:{level};\n\
				 #NOTES:\n\
				 {notes}\
				 ;\n"
			));
		}
		s.into_bytes()
	}

	fn fractured_fixture() -> Vec<Vec<u8>> {
		let bytes = make_ssc(&[
			("dance-single", "Easy chart", "easy", 3, "0000\n"),
			("dance-single", "Hard chart", "hard", 9, "0000\n"),
			("dance-single", "Alpha", "edit", 12, "0000\n"),
		]);
		fracture_bytes(&bytes)
			.unwrap()
			.into_iter()
			.map(|b| b.into_vec())
			.collect()
	}

	#[test]
	fn fracture_produces_one_chart_per_file() {
		let fractured = fracture_bytes(&make_ssc(&[
			("dance-single", "", "easy", 3, "0000\n"),
			("dance-single", "", "hard", 8, "0000\n"),
		]))
		.unwrap();

		assert_eq!(fractured.len(), 2);
		for chart in fractured {
			let msd = sm_msd::from_bytes(&chart);
			assert_eq!(msd.all_with_tag("NOTEDATA").len(), 1);
			assert_eq!(msd.all_with_tag("TITLE").len(), 1);
		}
	}

	#[test]
	fn fracture_rejects_zero_charts() {
		assert_eq!(
			fracture_bytes(b"#TITLE:Empty;\n").unwrap_err(),
			FractureError::NoCharts
		);
	}

	#[test]
	fn meld_sorts_charts_by_difficulty() {
		let mut fractured = fractured_fixture();
		fractured.reverse();

		let melded = String::from_utf8(meld_bytes(&fractured).unwrap().into_vec()).unwrap();
		let easy = melded.find("Easy chart").unwrap();
		let hard = melded.find("Hard chart").unwrap();
		let edit = melded.find("Alpha").unwrap();
		assert!(easy < hard && hard < edit);
	}

	#[test]
	fn meld_requires_single_chart_inputs() {
		let input = make_ssc(&[
			("dance-single", "A", "easy", 3, "0000\n"),
			("dance-single", "B", "hard", 9, "0000\n"),
		]);
		assert_eq!(
			meld_bytes(&[input]),
			Err(MeldError::InvalidFracture {
				input_index: 0,
				notedata_count: 2,
			})
		);
	}
}
