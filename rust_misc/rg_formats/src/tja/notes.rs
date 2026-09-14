use std::sync::OnceLock;

use regex::Regex;

use super::TICKS_PER_MEASURE;
use super::types::{BranchKind, Note, NoteKind};

fn command_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	// Exact pattern from CDTX.cs: command must be uppercase [A-Z]+ after '#'
	RE.get_or_init(|| Regex::new(r"^(#[A-Z]+)(?:\s?)(.+?)?$").expect("command regex is valid"))
}

fn branch_start_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	RE.get_or_init(|| {
		Regex::new(r"^([^,\s]+)\s*,\s*([^,\s]+)\s*,\s*([^,\s]+)$")
			.expect("branchstart regex is valid")
	})
}

/// Map a single note character to a `NoteKind`.
///
/// Returns `None` for rest (`'0'`) and invalid characters.
///
/// Exact `CharConvertNote` mapping from CDTX.cs:
/// - `'9'` maps to `Balloon` (potato → balloon, 2017.01.30 change).
/// - Only uppercase `A`, `B`, `F` are valid; lowercase variants are invalid.
pub(crate) fn char_to_note_kind(c: char) -> Option<NoteKind> {
	match c {
		'0' => None, // rest: advances time, no chip
		'1' => Some(NoteKind::Don),
		'2' => Some(NoteKind::Ka),
		'3' => Some(NoteKind::DonLarge),
		'4' => Some(NoteKind::KaLarge),
		'5' => Some(NoteKind::Roll),
		'6' => Some(NoteKind::RollLarge),
		'7' => Some(NoteKind::Balloon),
		'8' => Some(NoteKind::RollEnd),
		'9' => Some(NoteKind::Balloon), // potato → balloon (TJAPlayer3 2017.01.30)
		'A' => Some(NoteKind::HandHeld),
		'B' => Some(NoteKind::Special),
		'F' => Some(NoteKind::AdLib),
		_ => None, // invalid (lowercase a/b/f are invalid)
	}
}

/// Parse notes and events from a text block containing a `#START … #END` section.
///
/// Only the first `#START` / `#END` pair is processed. BPM changes, measure
/// changes, and other `#` commands are emitted as events at the current tick.
///
/// ## Tick calculation
///
/// For note at character index `i` in a measure row of `w` characters:
/// ```text
/// tick = measure_idx * 384 + (384 * i) / w
/// ```
/// Matching TJAPlayer3's `(int)(n現在の小節数 * 384.0 + (384.0 * n) / n文字数)`.
pub(super) fn parse_notes_from_text(
	text: &str,
	initial_bpm: f64,
	notes: &mut Vec<Note>,
	has_branches: &mut bool,
) {
	let mut in_chart = false;
	let mut measure_idx: u32 = 1; // starts at 1 (TJAPlayer3 n現在の小節数 init = 1)
	let mut accumulated = String::new();
	let mut current_bpm = initial_bpm;
	let mut measure_num = 4.0f64;
	let mut measure_den = 4.0f64;

	for line in text.lines() {
		// Trim leading/trailing whitespace from each line before processing.
		let line = line.trim();

		if line.is_empty() {
			continue;
		}

		if !in_chart {
			// Look for any #START variant (#START, #START P1, #START P2).
			if line.starts_with("#START") {
				in_chart = true;
			}
			continue;
		}

		// #END terminates the chart block.
		if line.starts_with("#END") {
			break;
		}

		// Lines starting with '#' are commands.
		if line.starts_with('#') {
			if let Some(caps) = command_re().captures(line) {
				let cmd = caps.get(1).map_or("", |m| m.as_str());
				let arg = caps.get(2).map_or("", |m| m.as_str()).trim();
				process_command(
					cmd,
					arg,
					measure_idx,
					notes,
					&mut current_bpm,
					&mut measure_num,
					&mut measure_den,
					has_branches,
				);
			}
			continue;
		}

		// Note row: characters are note digits; ',' terminates the measure.
		//
		// Lines without a comma accumulate into the next measure (multi-line
		// measure support, matching TJAPlayer3's accumulator logic).
		let (note_chars, has_comma) = match line.find(',') {
			Some(pos) => (&line[..pos], true),
			None => (line, false),
		};

		accumulated.push_str(note_chars);

		if has_comma {
			let measure_text = std::mem::take(&mut accumulated);
			let width = measure_text.chars().count();
			for (i, ch) in measure_text.chars().enumerate() {
				let Some(kind) = char_to_note_kind(ch) else {
					continue;
				};
				let Some(offset) = (TICKS_PER_MEASURE as usize * i).checked_div(width) else {
					continue;
				};
				let tick = measure_idx * TICKS_PER_MEASURE + offset as u32;
				notes.push(Note {
					measure: measure_idx,
					tick,
					kind,
				});
			}
			measure_idx += 1;
		}
	}
}

/// Process a single `#COMMAND` within a chart block.
///
/// Emits event notes where appropriate. Mirrors `t命令を挿入する` in CDTX.cs.
#[expect(clippy::too_many_arguments)]
fn process_command(
	cmd: &str,
	arg: &str,
	measure_idx: u32,
	notes: &mut Vec<Note>,
	bpm: &mut f64,
	measure_num: &mut f64,
	measure_den: &mut f64,
	has_branches: &mut bool,
) {
	let tick = measure_idx * TICKS_PER_MEASURE;

	match cmd {
		"#BPMCHANGE" => {
			// No comma→dot fix here (unlike the global BPM header).
			if let Ok(new_bpm) = arg.parse::<f64>() {
				*bpm = new_bpm;
				notes.push(Note {
					measure: measure_idx,
					tick,
					kind: NoteKind::BpmChange(new_bpm),
				});
			}
		}
		"#MEASURE" => {
			if let Some(slash) = arg.find('/') {
				let num_str = arg[..slash].trim();
				let den_str = arg[slash + 1..].trim();
				if let (Ok(num), Ok(den)) = (num_str.parse::<u32>(), den_str.parse::<u32>())
					&& den > 0
				{
					*measure_num = num as f64;
					*measure_den = den as f64;
					notes.push(Note {
						measure: measure_idx,
						tick,
						kind: NoteKind::Measure {
							numerator: num,
							denominator: den,
						},
					});
				}
			}
		}
		"#DELAY" => {
			if let Ok(secs) = arg.parse::<f64>() {
				notes.push(Note {
					measure: measure_idx,
					tick,
					kind: NoteKind::Delay(secs),
				});
			}
		}
		"#GOGOSTART" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::GogoStart,
			});
		}
		"#GOGOEND" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::GogoEnd,
			});
		}
		"#SECTION" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::Section,
			});
		}
		"#BRANCHSTART" => {
			*has_branches = true;
			if let Some(caps) = branch_start_re().captures(arg) {
				let kind_str = caps.get(1).map_or("", |m| m.as_str());
				let a = caps
					.get(2)
					.and_then(|m| m.as_str().parse::<f64>().ok())
					.unwrap_or(0.0);
				let b = caps
					.get(3)
					.and_then(|m| m.as_str().parse::<f64>().ok())
					.unwrap_or(0.0);
				let kind = match kind_str {
					"p" => BranchKind::Percent,
					"r" => BranchKind::Roll,
					"s" => BranchKind::Score,
					"d" => BranchKind::Drumroll,
					_ => BranchKind::Percent,
				};
				notes.push(Note {
					measure: measure_idx,
					tick,
					kind: NoteKind::BranchStart {
						kind,
						threshold_a: a,
						threshold_b: b,
					},
				});
			}
		}
		"#BRANCHEND" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::BranchEnd,
			});
		}
		"#BARLINEOFF" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::BarlineOff,
			});
		}
		"#BARLINEON" => {
			notes.push(Note {
				measure: measure_idx,
				tick,
				kind: NoteKind::BarlineOn,
			});
		}
		// #N / #E / #M — branch section switching (StartsWith("#N") bug reproduced:
		// #NEXTSONG also starts with "#N" and would match in TJAPlayer3's pre-scan).
		// We emit no note for these markers.
		//
		// #SCROLL, #LEVELHOLD, #LYRIC, #DIRECTION, #SUDDEN, #JPOSSCROLL,
		// #SENOTECHANGE, #NEXTSONG, #START, #END — not stored in the note IR.
		_ => {}
	}

	// Suppress unused-variable warnings for the timing fields we track but
	// don't yet use for time_ms computation.
	let _ = (measure_num, measure_den, bpm);
}
