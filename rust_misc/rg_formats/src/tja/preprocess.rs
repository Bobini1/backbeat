use std::sync::OnceLock;

use regex::Regex;

/// Header whitelist: the exact 32 names accepted by `regexForStrippingHeadingLines`.
///
/// Case-sensitive, matching TJAPlayer3's compile-time constant.
const HEADER_WHITELIST: &[&str] = &[
	"TITLE",
	"LEVEL",
	"BPM",
	"WAVE",
	"OFFSET",
	"BALLOON",
	"EXAM1",
	"EXAM2",
	"EXAM3",
	"BALLOONNOR",
	"BALLOONEXP",
	"BALLOONMAS",
	"SONGVOL",
	"SEVOL",
	"SCOREINIT",
	"SCOREDIFF",
	"COURSE",
	"STYLE",
	"GAME",
	"LIFE",
	"DEMOSTART",
	"SIDE",
	"SUBTITLE",
	"SCOREMODE",
	"GENRE",
	"MOVIEOFFSET",
	"BGIMAGE",
	"BGMOVIE",
	"HIDDENBRANCH",
	"GAUGEINCR",
	"#HBSCROLL",
	"#BMSCROLL",
];

fn comma_fix_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	// Multiline: matches ',' at the very start of any line → prepend '0'
	RE.get_or_init(|| Regex::new(r"(?m)^,").expect("comma fix regex is valid"))
}

fn comment_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	// Strip " *//.*" — spaces then '//' then everything to end-of-line
	RE.get_or_init(|| Regex::new(r" *//.*").expect("comment regex is valid"))
}

/// Strip non-whitelisted non-empty lines from the header portion.
///
/// Mirrors `regexForStrippingHeadingLines` from TJAPlayer3: removes every line
/// that is non-empty AND does not begin with one of the whitelisted keywords
/// (case-sensitive).
fn strip_non_whitelisted(header: &str) -> String {
	header
		.lines()
		.filter(|line| line.is_empty() || HEADER_WHITELIST.iter().any(|kw| line.starts_with(kw)))
		.collect::<Vec<_>>()
		.join("\n")
}

/// Run the full `t入力_V4` preprocessing pipeline and return cleaned lines.
pub(super) fn preprocess(text: &str) -> Vec<String> {
	// Step 1: normalise newlines and tabs; step 2: append trailing \n
	let text = text.replace("\r\n", "\n").replace('\t', " ");
	let text = if text.ends_with('\n') {
		text
	} else {
		format!("{text}\n")
	};

	// Step 3: comma-line fix — ^, → 0, (multiline)
	let text = comma_fix_re().replace_all(&text, "0,").into_owned();

	// Steps 4–6: split at first #START; strip non-whitelisted from header; rejoin
	let text = if let Some(start_idx) = text.find("#START") {
		let header = &text[..start_idx];
		let chart = &text[start_idx..];
		let clean_header = strip_non_whitelisted(header);
		format!("{clean_header}\n{chart}")
	} else {
		// No #START: skip the header strip step (TJAPlayer3 would throw here;
		// we handle it gracefully — still parse whatever headers exist)
		text
	};

	// Steps 7–8: line-split (RemoveEmptyEntries), strip comments, drop empty
	text.split('\n')
		.filter(|line| !line.is_empty())
		.map(|line| comment_re().replace(line, "").to_string())
		.filter(|line| !line.is_empty())
		.collect()
}
