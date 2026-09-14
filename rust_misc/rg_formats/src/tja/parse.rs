use super::header::{apply_header_line, convert_course};
use super::notes::parse_notes_from_text;
use super::preprocess::preprocess;
use super::types::{Chart, Course, Difficulty, Metadata, Note};

/// Parse a preprocessed + line-joined TJA text into a [`Chart`].
pub(super) fn parse_text(text: &str) -> Chart {
	let lines = preprocess(text);
	let joined = lines.join("\n");

	// Per-course state arrays (indexed by Difficulty index 0..=6).
	let mut meta = Metadata::default();
	let mut active: usize = 3; // default active course: Oni
	let mut level = [0u32; 7];
	let mut balloon_nor: [Vec<u32>; 7] = std::array::from_fn(|_| Vec::new());
	let mut balloon_exp: [Vec<u32>; 7] = std::array::from_fn(|_| Vec::new());
	let mut balloon_mas: [Vec<u32>; 7] = std::array::from_fn(|_| Vec::new());
	let mut score_init = [[300u16, 1000u16]; 7];
	let mut score_diff = [120u16; 7];
	let mut has_branches = [false; 7];
	let mut notes: [Vec<Note>; 7] = std::array::from_fn(|_| Vec::new());
	let mut present = [false; 7];

	// Split on the literal string "COURSE:" — exact tコースで譜面を分割する behavior.
	// Case-sensitive; not a regex.
	let segs: Vec<&str> = joined.split("COURSE:").collect();

	// Segment [0] is always the preamble (content before the first "COURSE:").
	for line in segs[0].lines() {
		apply_header_line(
			line,
			&mut meta,
			&mut active,
			&mut level,
			&mut balloon_nor,
			&mut balloon_exp,
			&mut balloon_mas,
			&mut score_init,
			&mut score_diff,
			&mut has_branches,
		);
	}

	if segs.len() == 1 {
		// No "COURSE:" found — entire text is Oni (index 3).
		// Only mark it present if there is at least a #START in the text.
		if joined.contains("#START") {
			present[3] = true;
			parse_notes_from_text(segs[0], meta.bpm, &mut notes[3], &mut has_branches[3]);
		}
	} else {
		for seg in &segs[1..] {
			// Read course name: characters until the first '\n'.
			let nl = seg.find('\n').unwrap_or(seg.len());
			let name = seg[..nl].trim();
			let body = &seg[nl..];

			let idx = convert_course(name);
			active = idx;
			present[idx] = true;

			// Parse per-course headers from this segment's body.
			for line in body.lines() {
				apply_header_line(
					line,
					&mut meta,
					&mut active,
					&mut level,
					&mut balloon_nor,
					&mut balloon_exp,
					&mut balloon_mas,
					&mut score_init,
					&mut score_diff,
					&mut has_branches,
				);
			}

			// Parse notes from the first #START…#END block in this course body.
			parse_notes_from_text(body, meta.bpm, &mut notes[idx], &mut has_branches[idx]);
		}
	}

	// Assemble Course structs for each present slot, preserving difficulty order.
	let courses: Vec<Course> = (0..7usize)
		.filter(|&i| present[i])
		.map(|i| Course {
			difficulty: Difficulty::from_index(i),
			level: level[i],
			balloon_normal: std::mem::take(&mut balloon_nor[i]),
			balloon_expert: std::mem::take(&mut balloon_exp[i]),
			balloon_master: std::mem::take(&mut balloon_mas[i]),
			score_init: score_init[i],
			score_diff: score_diff[i],
			has_branches: has_branches[i],
			notes: std::mem::take(&mut notes[i]),
		})
		.collect();

	Chart {
		metadata: meta,
		courses,
	}
}
