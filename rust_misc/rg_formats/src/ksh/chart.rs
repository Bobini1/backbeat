use super::{
	LoadError,
	body::{BodyOption, Measure, MeasureEvent},
	definition::Definition,
	header::Header,
	raw_headers::RawHeaders,
	row::Row,
};

// ── Chart ─────────────────────────────────────────────────────────────────────

/// A fully-parsed KSH chart file.
#[derive(Debug, Clone)]
pub struct Chart {
	/// Header declarations in source order.
	pub raw_headers: RawHeaders,
	pub header: Header,
	/// One entry per measure (section between consecutive `--` bar lines).
	pub measures: Vec<Measure>,
	/// `#define_fx` and `#define_filter` entries.
	pub definitions: Vec<Definition>,
}

impl Chart {
	pub(super) fn parse(raw: &[u8]) -> Result<Self, LoadError> {
		let encoding = super::KshEncoding::detect(raw);
		let text = encoding.decode(raw)?;
		let raw_headers = RawHeaders::from_text(&text);
		let header = Header::parse(&raw_headers);

		let (measures, definitions) = parse_body(&text)?;

		Ok(Self {
			raw_headers,
			header,
			measures,
			definitions,
		})
	}
}

// ── Body parser ───────────────────────────────────────────────────────────────
//
// Returns data for two unrelated types (Measure + Definition), so it stays a
// free function rather than being a method on either.

pub(super) fn parse_body(raw: &str) -> Result<(Vec<Measure>, Vec<Definition>), LoadError> {
	// Collect non-empty, CRLF-normalised lines.
	let lines: Vec<&str> = raw
		.lines()
		.map(|l| l.trim_end_matches('\r'))
		.filter(|l| !l.is_empty())
		.collect();

	// Locate the first bar line — it divides header from body.
	let first_bar = lines
		.iter()
		.position(|l| *l == "--")
		.ok_or(LoadError::NoBarLine)?;

	let mut measures: Vec<Measure> = Vec::new();
	let mut current = Measure { events: Vec::new() };
	let mut definitions: Vec<Definition> = Vec::new();

	for (line_num, line) in lines[first_bar + 1..].iter().enumerate() {
		if line.starts_with("//") {
			continue;
		}
		if *line == "--" {
			measures.push(current);
			current = Measure { events: Vec::new() };
			continue;
		}
		if let Some(def) = Definition::try_parse(line) {
			definitions.push(def);
			continue;
		}
		if Row::is_chart_line(line) {
			let row = Row::parse(line, line_num + 1)?;
			current.events.push(MeasureEvent::Row(row));
			continue;
		}
		if let Some((key, value)) = line.split_once('=') {
			let opt = BodyOption::parse(key, value);
			current.events.push(MeasureEvent::Option(opt));
		}
	}

	// Trailing measure (well-formed files end with `--`, but handle gracefully).
	if !current.events.is_empty() {
		measures.push(current);
	}

	Ok((measures, definitions))
}
