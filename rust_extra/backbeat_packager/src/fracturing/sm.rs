//! Fracture and meld `.sm` files.

use rg_formats::sm_msd::{self, MsdElement, MsdFile};
use sha2::{Digest, Sha256};

use super::FractureError;

/// Errors that can occur while melding fractured SM files.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MeldError {
	/// A fracture must contain exactly one chart.
	#[error(
		"fractured SM input {input_index} contains {notes_count} #NOTES sections, expected one"
	)]
	InvalidFracture {
		/// The zero-based position of the incompatible input.
		input_index: usize,
		/// The number of chart sections in the input.
		notes_count: usize,
	},

	/// Fractures must have identical shared metadata.
	#[error("fractured SM input {input_index} has incompatible shared metadata")]
	IncompatibleMetadata {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},

	/// The chart has no difficulty field.
	#[error("fractured SM input {input_index} has a #NOTES section without a difficulty")]
	MissingDifficulty {
		/// The zero-based position of the incompatible input.
		input_index: usize,
	},
}

/// Split a multi-chart SM into one SM blob per `#NOTES` section.
pub(crate) fn fracture_bytes(bytes: &[u8]) -> Result<Vec<Box<[u8]>>, FractureError> {
	let msd = sm_msd::from_bytes(bytes);
	let header = msd
		.elements
		.iter()
		.filter(|element| &*element.tag != b"NOTES")
		.cloned()
		.collect::<Vec<_>>();

	let fractured = msd
		.elements
		.into_iter()
		.filter(|element| &*element.tag == b"NOTES")
		.map(|notes| {
			let mut elements = header.clone();
			elements.push(notes);
			sm_msd::serialize_msd_elements(elements)
		})
		.collect::<Vec<_>>();

	if fractured.is_empty() {
		return Err(FractureError::NoCharts);
	}

	Ok(fractured)
}

/// Recombine compatible, single-chart SM files into one canonical SM file.
pub(crate) fn meld_bytes(inputs: &[Vec<u8>]) -> Result<Box<[u8]>, MeldError> {
	let mut parent_id = None;
	let mut header = None;
	let mut charts = Vec::with_capacity(inputs.len());

	for (input_index, input) in inputs.iter().enumerate() {
		let msd = sm_msd::from_bytes(input);
		let notes = msd.all_with_tag("NOTES");
		if notes.len() != 1 {
			return Err(MeldError::InvalidFracture {
				input_index,
				notes_count: notes.len(),
			});
		}

		let shared_elements = non_notes_elements(&msd);
		let shared_id = fracture_parent_id(&shared_elements);
		if let Some(expected_parent_id) = parent_id {
			if expected_parent_id != shared_id {
				return Err(MeldError::IncompatibleMetadata { input_index });
			}
		} else {
			parent_id = Some(shared_id);
			header = Some(shared_elements);
		}

		let notes = notes[0].clone();
		let difficulty = chart_difficulty_key(&notes, input_index)?;
		let notes_id = Sha256::digest(sm_msd::serialize_msd_elements(vec![notes.clone()]));
		charts.push((difficulty, notes_id, notes));
	}

	charts.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

	let mut elements = header.unwrap_or_default();
	elements.extend(charts.into_iter().map(|(_, _, notes)| notes));
	Ok(sm_msd::serialize_msd_elements(elements))
}

fn non_notes_elements(msd: &MsdFile) -> Vec<MsdElement> {
	msd.elements
		.iter()
		.filter(|element| !is_notes(element))
		.cloned()
		.collect()
}

fn is_notes(element: &MsdElement) -> bool {
	&*element.tag == b"NOTES"
}

fn fracture_parent_id(elements: &[MsdElement]) -> [u8; 32] {
	Sha256::digest(sm_msd::serialize_msd_elements(elements.to_vec())).into()
}

fn chart_difficulty_key(
	notes: &MsdElement,
	input_index: usize,
) -> Result<(u8, Box<[u8]>), MeldError> {
	let Some(difficulty) = notes.values.get(2) else {
		return Err(MeldError::MissingDifficulty { input_index });
	};

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
		notes.values.get(1).cloned().unwrap_or_default()
	} else {
		Box::default()
	};

	Ok((rank, edit_name))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_sm(notes_count: usize) -> Vec<u8> {
		let mut s = String::from(
			"#TITLE:Test Song;\n\
			 #ARTIST:Test Artist;\n\
			 #MUSIC:song.ogg;\n",
		);
		for i in 0..notes_count {
			s.push_str(&format!(
				"#NOTES:\n\
				     dance-single:\n\
				     Author:\n\
				     Hard:\n\
				     {}:\n\
				     0,0,0,0,0:\n\
				0000\n\
				;\n",
				10 + i
			));
		}
		s.into_bytes()
	}

	fn fractured_fixture(bpms: &str) -> Vec<Vec<u8>> {
		[
			"Hard chart:Hard:9:0:0000",
			"Alpha:Edit:10:0:0010",
			"Easy chart:Easy:3:0:0100",
		]
		.into_iter()
		.map(|notes| {
			format!("#TITLE:Test;\n#BPMS:{bpms};\n#NOTES:dance-single:{notes};\n").into_bytes()
		})
		.collect()
	}

	#[test]
	fn fracture_produces_one_chart_per_file() {
		let fractured = fracture_bytes(&make_sm(3)).unwrap();

		assert_eq!(fractured.len(), 3);
		for chart in fractured {
			let msd = sm_msd::from_bytes(&chart);
			assert_eq!(msd.all_with_tag("TITLE").len(), 1);
			assert_eq!(msd.all_with_tag("NOTES").len(), 1);
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
		let mut fractured = fractured_fixture("0=120");
		fractured.reverse();

		let melded = String::from_utf8(meld_bytes(&fractured).unwrap().into_vec()).unwrap();
		let easy = melded.find("Easy chart").unwrap();
		let hard = melded.find("Hard chart").unwrap();
		let edit = melded.find("Alpha").unwrap();
		assert!(easy < hard && hard < edit);
	}

	#[test]
	fn meld_rejects_incompatible_shared_metadata() {
		let first = fractured_fixture("0=120")[0].clone();
		let second = fractured_fixture("0=240")[1].clone();

		assert_eq!(
			meld_bytes(&[first, second]),
			Err(MeldError::IncompatibleMetadata { input_index: 1 })
		);
	}

	#[test]
	fn meld_requires_single_chart_inputs() {
		let input = fractured_fixture("0=120").into_iter().flatten().collect();
		assert_eq!(
			meld_bytes(&[input]),
			Err(MeldError::InvalidFracture {
				input_index: 0,
				notes_count: 3,
			})
		);
	}
}
