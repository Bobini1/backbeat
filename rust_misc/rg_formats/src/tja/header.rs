use super::types::Metadata;

/// Map a COURSE name string to a 0-based difficulty index.
///
/// Exact `strConvertCourse` behavior: case-insensitive substring match first,
/// then numeric parse, defaulting to 3 (Oni).
pub(crate) fn convert_course(name: &str) -> usize {
	let lower = name.to_ascii_lowercase();
	if lower.contains("easy") {
		return 0;
	}
	if lower.contains("normal") {
		return 1;
	}
	if lower.contains("hard") {
		return 2;
	}
	if lower.contains("oni") {
		return 3;
	}
	if lower.contains("edit") {
		return 4;
	}
	if lower.contains("tower") {
		return 5;
	}
	if lower.contains("dan") {
		return 6;
	}
	if let Ok(n @ 0..=6) = name.trim().parse::<usize>() {
		return n;
	}
	3 // default: Oni
}

/// Parse a single header line, updating metadata and per-course arrays.
///
/// Mirrors `t入力_行解析ヘッダ` in CDTX.cs.
#[expect(clippy::too_many_arguments)]
pub(super) fn apply_header_line(
	line: &str,
	meta: &mut Metadata,
	active: &mut usize,
	level: &mut [u32; 7],
	balloon_normal: &mut [Vec<u32>; 7],
	balloon_expert: &mut [Vec<u32>; 7],
	balloon_master: &mut [Vec<u32>; 7],
	score_init: &mut [[u16; 2]; 7],
	score_diff: &mut [u16; 7],
	has_branches: &mut [bool; 7],
) {
	// #BRANCHSTART triggers has_branches before the ':' split (CDTX.cs special case)
	if line.starts_with("#BRANCHSTART") {
		has_branches[*active] = true;
	}

	// Split on first ':' only; both halves are trimmed
	let Some(colon) = line.find(':') else { return };
	let name = line[..colon].trim();
	let param = line[colon + 1..].trim();

	if name.is_empty() {
		return;
	}

	match name {
		"TITLE" => {
			// TJAPlayer3: join all ':'-split parts of the full line, then strip
			// the leading "TITLE" prefix (5 chars). Effectively: everything after
			// the first ':' preserving embedded colons.
			meta.title = Some(param.to_owned());
		}
		"SUBTITLE" => {
			// Strip leading "--" or "++" display-mode markers.
			let s = if param.starts_with("--") || param.starts_with("++") {
				&param[2..]
			} else {
				param
			};
			meta.subtitle = Some(s.to_owned());
		}
		"BPM" => {
			// Replace ',' with '.' before parsing (European decimal separator).
			// Only the global BPM header does this; #BPMCHANGE does NOT.
			let val = param.replace(',', ".");
			if let Ok(bpm) = val.parse::<f64>() {
				meta.bpm = bpm;
			}
		}
		"WAVE" => {
			// First WAVE declaration wins; extras are silently ignored.
			if meta.wave.is_none() && !param.is_empty() {
				meta.wave = Some(param.to_owned());
			}
		}
		"OFFSET" => {
			if let Ok(secs) = param.parse::<f64>() {
				meta.offset_ms = (secs * 1000.0) as i32;
			}
		}
		"MOVIEOFFSET" => {
			if let Ok(secs) = param.parse::<f64>() {
				meta.movie_offset_ms = Some((secs * 1000.0) as i32);
			}
		}
		"BALLOON" | "BALLOONNOR" => {
			balloon_normal[*active] = parse_balloon(param);
		}
		"BALLOONEXP" => {
			balloon_expert[*active] = parse_balloon(param);
		}
		"BALLOONMAS" => {
			balloon_master[*active] = parse_balloon(param);
		}
		"SCOREINIT" => {
			let parts: Vec<&str> = param.split(',').collect();
			if let Some(v) = parts.first().and_then(|s| s.trim().parse::<u16>().ok()) {
				score_init[*active][0] = v;
			}
			if let Some(v) = parts.get(1).and_then(|s| s.trim().parse::<u16>().ok()) {
				score_init[*active][1] = v;
			}
		}
		"SCOREDIFF" => {
			if let Ok(v) = param.parse::<u16>() {
				score_diff[*active] = v;
			}
		}
		"COURSE" => {
			*active = convert_course(param);
		}
		"LEVEL" => {
			// `(double) as u32` matches TJAPlayer3's `(int)Convert.ToDouble(...)`.
			if let Ok(v) = param.parse::<f64>() {
				level[*active] = v as u32;
			}
		}
		"GENRE" => {
			if !param.is_empty() && meta.genre.is_none() {
				meta.genre = Some(param.to_owned());
			}
		}
		"DEMOSTART" => {
			if let Ok(secs) = param.parse::<f64>() {
				meta.demo_start = Some(secs);
			}
		}
		"BGMOVIE" => {
			if meta.bg_movie.is_none() && !param.is_empty() {
				meta.bg_movie = Some(param.to_owned());
			}
		}
		"BGIMAGE" => {
			if meta.bg_image.is_none() && !param.is_empty() {
				meta.bg_image = Some(param.to_owned());
			}
		}
		"HIDDENBRANCH" if !param.is_empty() => {
			meta.hidden_branch = true;
		}
		// STYLE, GAME, LIFE, SIDE, SCOREMODE, SONGVOL, SEVOL, GAUGEINCR,
		// HEADSCROLL, EXAM1/2/3, #HBSCROLL, #BMSCROLL — parsed by TJAPlayer3
		// but not stored in the backbeat IR.
		_ => {}
	}
}

/// Parse `BALLOON:` / `BALLOONNOR:` comma-separated hit counts.
///
/// Stops on empty segments (matching TJAPlayer3's `ParseBalloon`).
pub(crate) fn parse_balloon(param: &str) -> Vec<u32> {
	let mut counts = Vec::new();
	for part in param.split(',') {
		if part.is_empty() {
			break;
		}
		if let Ok(n) = part.trim().parse::<u32>() {
			counts.push(n);
		}
	}
	counts
}
