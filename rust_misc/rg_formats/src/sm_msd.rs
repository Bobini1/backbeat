//! Parsing, Processing and serializing of `.msd` files. This is the file format that
//! underpins `.sm` and `.ssc`.
//!
//! **You might be looking for an `.sm` parser. Use [`sm`][crate::sm] for that.**
//!
//! MSD or "Music Score Data" is an extremely old format for very early dance pad
//! emulators. While nowadays the `.msd` extension is effectively dead, it lives on in
//! `.sm` and `.ssc`, as its grammar underpins both.
//!
//! This module implements a StepMania-accurate MSD parser.
//!
//! # What is MSD?
//!
//! An average MSD file looks like this:
//!
//! ```msd
//! #TITLE:Hello World;
//! #ARTIST:McLusky;
//! //--------------- dance-single - Todestrieb ----------------
//! #NOTES:
//!      dance-single:
//!      Todestrieb:
//!      Hard:
//!      15:
//!      0,0,0,0,0:
//! 0000
//! 1000
//! 0000
//! 1000;
//! ```
//!
//! Despite what you might intuit by looking at it, MSD parses into a 2-dimensional array.
//!
//! The `#` value indicates a tag. A file may have multiple of the same tag, and there are
//! formats implemented on top of MSD (like SSC) that depend on this behaviour.
//!
//! Subsequent colons — `:` — separate the params of a tag. For example, the `#NOTES`
//! field above parses into:
//!
//! ```json
//! ["dance-single", "Todestrieb", "Hard", "15", "0,0,0,0,0", "0000\n1000\n0000\n1000"]
//! ```
//!
//! The grammar is:
//!
//! ```ebnf
//! "#" tag_name (":" param)* ";"
//! ```
//!
//! The semicolon **always** terminates the current tag. Missing semicolons are the
//! one real error-correction that SM performs: if `#` appears as the first
//! non-whitespace character on a new line while inside a tag, SM treats it as an
//! implicit semicolon and starts a new tag. This lets the following parse:
//!
//! ```msd
//! #ARTIST:McLusky
//!   #Title:Without MSD I Am Nothing;
//! ```
//!
//! # Other Things
//!
//! MSD also supports `//` comments, and `\` as an escape character. This allows:
//!
//! ```msd
//! #TITLE\//:SongTitle\;
//! \ #ARTIST:McLusky
//! ```
//!
//! to parse as
//! ```json
//! ["TITLE//", "SongTitle"]
//! ["ARTIST", "McLusky"]
//! ```
//!
//! [sm]: crate::sm

use std::fmt::Debug;

use super::utils::ByteIterator;
use crate::utils::{ByteString, read};

/// An MSD file is a vector of [`MsdElement`]s. Tags may be repeated, and will *always*
/// be uppercased for SM compatibility.
#[derive(Debug, Clone, Default)]
pub struct MsdFile {
	/// The elements this MsdFile defined.
	pub elements: Vec<MsdElement>,
}

impl MsdFile {
	/// Make an empty MsdFile.
	pub fn new() -> Self {
		Self::default()
	}

	/// Get the first element with this tag.
	pub fn get_element(&self, tag: &str) -> Option<MsdElement> {
		let tag_upper = tag.to_ascii_uppercase();
		let tag_upper = tag_upper.as_bytes();

		for el in &self.elements {
			if *el.tag == *tag_upper {
				return Some(el.clone());
			}
		}

		None
	}

	/// Get the first instance of this tag, and get the first value.
	/// Common for things like
	/// #TITLE:Foo;
	///        ^^^
	///
	/// Returns Some("foo") (boxed and sliced whatever)
	pub fn get_first_value(&self, tag: &str) -> Option<Box<[u8]>> {
		let v = self.get_element(tag)?;

		match v.values.first() {
			Some(v) => {
				// empty strings count as none
				if v.is_empty() {
					return None;
				}

				Some(v.clone())
			}
			None => None,
		}
	}

	/// Get all elements with this tag. This is for things like #NOTES, which may
	/// appear multiple times.
	pub fn all_with_tag(&self, tag: &str) -> Vec<&MsdElement> {
		let tag_upper = tag.to_ascii_uppercase();
		let tag_upper = tag_upper.as_bytes();

		self.elements
			.iter()
			.filter(|f| *f.tag == *tag_upper)
			.collect()
	}
}

/// An element in the MSD format. This is parsed from data like:
///
/// ```txt
/// #TITLE:Hello World;
/// #ARTIST:Foo;
/// ```
#[derive(Clone)]
pub struct MsdElement {
	/// The first value in this element -- this is prefixed with a `#` and ALWAYS
	/// uppercased.
	pub tag: ByteString,
	/// The values this tag pointed to. MSD elements can have an arbitrary amount of
	/// elements tied to their tag:
	///
	/// ```txt
	/// #TAG:Value1:value2:Value3;
	/// ```
	pub values: Vec<ByteString>,
}

impl Debug for MsdElement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MsdElement")
			.field("tag", &String::from_utf8_lossy(&self.tag))
			.field(
				"values",
				&self
					.values
					.iter()
					.map(|f| String::from_utf8_lossy(f))
					.collect::<Vec<_>>(),
			)
			.finish()
	}
}

fn write_param(data: &mut Vec<u8>, param: &ByteString) {
	for byte in param.iter() {
		let byte = *byte;

		if matches!(byte, b':' | b';' | b'#' | b'\\') {
			data.push(b'\\');
		}

		data.push(byte);
	}
}

/// Given a list of [`MsdElement`]s, serialize them. This can be used to serialize SM
/// files, or SSC files, or whatnot.
pub fn serialize_msd_elements(elements: Vec<MsdElement>) -> Box<[u8]> {
	let mut data = vec![];

	for el in elements {
		data.push(b'#');

		write_param(&mut data, &el.tag);

		// of course, converting notes needs to be formatted specially for most
		// things to parse it properly.
		// thanks SM.

		if &*el.tag == b"NOTES" {
			for (index, value) in el.values.iter().enumerate() {
				// if last index, format specially
				if index == el.values.len() - 1 {
					data.extend(b":\n")
				} else {
					// otherwise, the SM formatter does a colon, a newline,
					// exactly *five* spaces, and then writes the data.
					data.extend(b":\n     ");
				}

				write_param(&mut data, value);
			}
		} else {
			for value in el.values {
				data.push(b':');

				write_param(&mut data, &value);
			}
		}

		data.push(b';');
		data.push(b'\n');
	}

	data.into()
}

/// SM's "MSD" format is the internal underpinning for the SM, SSC, DWI and SMA formats.
///
/// It is a list of tuples with the grammar:
///
/// ```ebnf
/// "#" tag_name (":" param)* ";"
/// ```
///
/// `;` terminates the current tag. The only error-correction is that a `#` appearing
/// as the first non-whitespace character on a new line acts as an implicit `;` and
/// starts a new tag. Repeated keys are allowed and important to the syntax (e.g.
/// multiple `#NOTES` entries in one file).
pub fn from_bytes(buf: &[u8]) -> MsdFile {
	parse_bytes(buf, true)
}

pub(crate) fn from_bytes_preserving_escapes(buf: &[u8]) -> MsdFile {
	parse_bytes(buf, false)
}

fn parse_bytes(buf: &[u8], unescape: bool) -> MsdFile {
	let mut elements: Vec<MsdElement> = vec![];
	let mut b_iter = ByteIterator::new(buf);

	loop {
		let (param_list, term_reason) = parse_param_list(&mut b_iter, unescape);

		if let Some(tag) = param_list.first() {
			elements.push(MsdElement {
				tag: tag.to_ascii_uppercase().into(),
				values: param_list.into_iter().skip(1).collect(),
			})
		}

		if matches!(term_reason, ParamListTermReason::Eof) {
			break;
		}
	}

	MsdFile { elements }
}

enum ParamListTermReason {
	Eof,
	Hash,
	/// A `;` terminated the element; the caller should keep scanning for the next `#`.
	Semicolon,
}

/// Parse "#TAG:VALUE:VALUE2;" into a vector of ["TAG", "VALUE", "VALUE2"]
/// Also returns the reason why we terminated.
fn parse_param_list(
	b_iter: &mut ByteIterator,
	unescape: bool,
) -> (Vec<ByteString>, ParamListTermReason) {
	let mut params = vec![];

	// firstly skip until values actually start
	loop {
		match b_iter.read() {
			// if we see a hash, values have started
			Some(b'#') => break,
			// if we hit EOF, return params
			None => return (params, ParamListTermReason::Eof),
			// otherwise, keep going
			_ => continue,
		}
	}

	loop {
		let (param, terminate) = parse_param(b_iter, unescape);

		match terminate {
			ParamTermReason::Hash => {
				// Walk back one: we've consumed the `#` but the next call to
				// `parse_param_list` needs to see it to know where the tag starts.
				b_iter.rewind();

				// Don't add an empty trailing param — prevents
				// `#ARTIST:FOO;` from yielding ["ARTIST", "FOO", ""].
				if !param.is_empty() {
					params.push(param);
				}

				return (params, ParamListTermReason::Hash);
			}
			ParamTermReason::Eof => {
				if !param.is_empty() {
					params.push(param);
				}

				return (params, ParamListTermReason::Eof);
			}
			ParamTermReason::Semicolon => {
				// `;` is the canonical element terminator (same as SM's `ReadingValue=false`).
				// Always push the param — even if empty — to match SM's behaviour where
				// `#SUBTITLE:;` yields one empty-string value.  SM uses `iProcessedLen != -1`
				// (i.e. "we have entered a param") for the same purpose.
				params.push(param);

				return (params, ParamListTermReason::Semicolon);
			}
			ParamTermReason::Colon => {}
		}

		params.push(param);
	}
}

enum ParamTermReason {
	Hash,
	Colon,
	Semicolon,
	Eof,
}

fn parse_param(b_iter: &mut ByteIterator, unescape: bool) -> (ByteString, ParamTermReason) {
	fn parse_val_inner(b_iter: &mut ByteIterator, unescape: bool) -> ByteString {
		let mut param = vec![];

		loop {
			let byte = read!(b_iter);

			match byte {
				b'\\' => {
					let next_byte = read!(b_iter);
					if !unescape {
						param.push(byte);
					}
					param.push(next_byte);
				}
				// since read!() increments the cursor
				// peek will be off-by-one.
				// comments are two /'s literally in a row.
				b'/' if b_iter.peek() == Some(&b'/') => {
					// it's a comment, read until newline or eof

					let mut chomp = read!(b_iter);
					while chomp != b'\n' {
						chomp = read!(b_iter);
					}

					// keep the newline
					while matches!(param.last(), Some(b' ' | b'\t')) {
						param.pop();
					}
					param.push(b'\n');
				}
				b'#' => {
					// a hash could be part of the value:
					// #TITLE:Magical #girl;
					// or could be the sign of a missing semicolon:
					// #TITLE:Magical
					// #ARTIST:Girl

					// this depends on whether we see a newline preceding this.
					// go backwards through what we've already read.
					for backtrack_byte in param.iter().rev() {
						if *backtrack_byte == b'\n' {
							// this was meant to be a separator, exit the function.
							return param.into();
						}

						if *backtrack_byte == b' ' || *backtrack_byte == b'\t' {
							// things like
							// #Title:foo
							//    #artist:bar
							// should still be understood as a missing semicolon
							continue;
						}

						// no, this # is part of the value literally
						break;
					}

					param.push(byte);
				}
				b':' | b';' => {
					// end of value
					return param.into();
				}
				_ => {
					// everything else is just part of the value
					param.push(byte)
				}
			}
		}

		param.into()
	}

	// trim all leading and trailing whitespace from the value.
	let trimmed = parse_val_inner(b_iter, unescape).trim_ascii().into();

	let t_reason = match b_iter.prev() {
		Some(b'#') => ParamTermReason::Hash,
		Some(b':') => ParamTermReason::Colon,
		Some(b';') => ParamTermReason::Semicolon,
		None => ParamTermReason::Eof,

		// should be impossible
		u => panic!("Unexpected terminator {u:?}"),
	};

	(trimmed, t_reason)
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;
	use crate::test_utils::test_file_read;

	macro_rules! test_parse_prm {
		($input:expr, $index:expr, $out:expr) => {{
			let mut biter = ByteIterator::new($input);
			biter.set_index($index);

			assert_eq!(
				std::str::from_utf8(&parse_param(&mut biter, true).0).unwrap(),
				$out
			)
		}};
	}

	#[test]
	fn p_val() {
		test_parse_prm!(b"#TITLE:AMONG US;", 1, "TITLE");
		test_parse_prm!(b"#TITLE:AMONG US;", 7, "AMONG US");
	}

	#[test]
	fn inline_comments_do_not_leave_trailing_whitespace() {
		test_parse_prm!(
			b"#NOTEDATA:
0000
0100
0000
0001, // measure 1
0000
0000
0000
0000
, // measure 2
;",
			10,
			"0000
0100
0000
0001,
0000
0000
0000
0000
,"
		);
	}

	#[test]
	fn newlines() {
		test_parse_prm!(
			b"#TITLE: New Line
Here;",
			7,
			"New Line
Here"
		);

		test_parse_prm!(
			b"#TITLE: Missing Semicolon
#ARTIST: foo;",
			7,
			"Missing Semicolon"
		);

		test_parse_prm!(
			b"#TITLE: Missing Semicolon wspace
	  #ARTIST: foo;",
			7,
			"Missing Semicolon wspace"
		);

		test_parse_prm!(
			b"#TITLE: Missing Semicolon
	  heyo #ARTIST: foo;",
			7,
			"Missing Semicolon
	  heyo #ARTIST"
		);
	}

	#[test]
	fn escape() {
		test_parse_prm!(br"#TITLE: foo\:bar;", 7, r"foo:bar");
		test_parse_prm!(br"#TITLE: foo\\:bar;", 7, r"foo\");

		test_parse_prm!(br"#TITLE: foo\\\:bar;", 7, r"foo\:bar");
		test_parse_prm!(
			br"#TITLE: foo\
#artist",
			7,
			r"foo"
		);
	}

	#[test]
	fn multival() {
		test_parse_prm!(b"#MULTI:VALUE:VALUE2;", 7, "VALUE");
		test_parse_prm!(b"#MULTI:VALUE:VALUE2;", 13, "VALUE2");
	}

	#[test]
	fn comment() {
		test_parse_prm!(b"#TITLE: A/BCD;", 7, "A/BCD");
		test_parse_prm!(b"#TITLE: A//BCD;", 7, "A");
	}

	macro_rules! test_parse_plist {
		($input:expr, $out:expr) => {{
			let mut biter = ByteIterator::new($input);

			assert_eq!(
				parse_param_list(&mut biter, true)
					.0
					.iter()
					.map(|b| std::str::from_utf8(&b).unwrap())
					.collect::<Vec<_>>(),
				$out
			);
		}};
	}

	#[test]
	fn list() {
		// Basic: tag with no trailing semi
		test_parse_plist!(b"#TITLE:FOO", vec!["TITLE", "FOO"]);

		// `;` terminates the element — "BAR:BAZ" is outside any element and is dropped.
		test_parse_plist!(b"#TITLE:FOO;BAR:BAZ", vec!["TITLE", "FOO"]);

		// `;` on its own line also terminates.
		test_parse_plist!(
			b"#TITLE:FOO;
#ARTIST:AMONG_US;",
			vec!["TITLE", "FOO"]
		);

		// `;` followed immediately by `#` (no newline): `;` ends the element, the
		// `#` starts a NEW element that parse_param_list does NOT return.
		test_parse_plist!(b"#TITLE:FOO;#BAR:BAZ", vec!["TITLE", "FOO"]);

		// `;` mid-line followed by `#` on the next line: `;` ends the element.
		test_parse_plist!(
			b"#TITLE:FOO
;#BAR:BAZ",
			vec!["TITLE", "FOO"]
		);

		// Missing semicolon: `#` at start of new line acts as implicit `;`.
		test_parse_plist!(
			b"#TITLE:FOO
#BAR:BAZ",
			vec!["TITLE", "FOO"]
		);

		// Same with a trailing colon before the newline.
		test_parse_plist!(
			b"#TITLE:FOO:
#BAR:BAZ",
			vec!["TITLE", "FOO"]
		);

		// Escaped-space before `#` on a new line: still treated as missing semicolon.
		test_parse_plist!(
			br"#TITLE:FOO:
\ #BAR:BAZ",
			vec!["TITLE", "FOO"]
		);
	}

	#[test]
	fn semicolon_is_terminator() {
		// Verify from_bytes: `;` ends the element; subsequent content starts fresh.
		let res = from_bytes(b"#TITLE:FOO;#ARTIST:BAR;");
		assert_eq!(res.elements.len(), 2);
		assert_eq!(&*res.elements[0].tag, b"TITLE");
		assert_eq!(&*res.elements[0].values[0], b"FOO");
		assert_eq!(&*res.elements[1].tag, b"ARTIST");
		assert_eq!(&*res.elements[1].values[0], b"BAR");
	}

	#[test]
	fn preserving_escapes_keeps_dwi_paths() {
		let bytes = br"#FILE:music\track.ogg;#TITLE:Left\:Right\;Still;";
		let unescaped = from_bytes(bytes);
		let preserved = from_bytes_preserving_escapes(bytes);

		assert_eq!(
			unescaped.get_first_value("FILE").as_deref(),
			Some(b"musictrack.ogg" as &[u8])
		);
		assert_eq!(
			preserved.get_first_value("FILE").as_deref(),
			Some(b"music\\track.ogg" as &[u8])
		);
		assert_eq!(
			unescaped.get_first_value("TITLE").as_deref(),
			Some(b"Left:Right;Still" as &[u8])
		);
		assert_eq!(
			preserved.get_first_value("TITLE").as_deref(),
			Some(b"Left\\:Right\\;Still" as &[u8])
		);
	}

	#[test]
	fn semicolon_mid_element_is_not_separator() {
		// `#TITLE:FOO;BAR:BAZ` must produce ONE element ["TITLE","FOO"],
		// not ["TITLE","FOO","BAR","BAZ"]. "BAR:BAZ" is outside any element.
		let res = from_bytes(b"#TITLE:FOO;BAR:BAZ\n#ARTIST:ZK;");
		assert_eq!(res.elements.len(), 2, "expected exactly 2 elements");
		assert_eq!(&*res.elements[0].tag, b"TITLE");
		assert_eq!(res.elements[0].values.len(), 1);
		assert_eq!(&*res.elements[0].values[0], b"FOO");
		assert_eq!(&*res.elements[1].tag, b"ARTIST");
	}

	fn full_comp(buf: &[u8], expected: Vec<Vec<&str>>) {
		let parse_res = from_bytes(buf);

		for (i, values) in expected.iter().enumerate() {
			let e = parse_res.elements[i].clone();

			assert_eq!(
				std::str::from_utf8(&e.tag).unwrap(),
				values.first().unwrap().to_owned()
			);

			assert_eq!(
				e.values
					.into_iter()
					.map(|f| std::str::from_utf8(&f).unwrap().to_owned())
					.collect::<Vec<String>>(),
				values
					.iter()
					.skip(1)
					// lolwhat
					.map(|f| f.to_owned().to_owned())
					.collect::<Vec<String>>()
			);
		}
	}

	#[test]
	fn full_parse() {
		let expected = vec![
			vec!["TITLE", "[11] [120] Le Perv (zk test ed.)"],
			vec!["ARTIST", "Carpenter Brut"],
			vec!["SUBTITLE", ""],
		];

		full_comp(&test_file_read("sm/zk_test.sm"), expected);
	}

	#[test]
	fn full_parse_simple() {
		let expected = vec![
			vec!["TITLE", "Song Title"],
			vec!["ARTIST", "Song Artist"],
			vec!["SUBTITLE", ""],
			vec!["WHITESPACE", "Foo"],
		];

		full_comp(
			br"#TITLE:Song Title;
#Artist:Song Artist;
#SUBTITLE:;
#WHITESPACE:   Foo;",
			expected,
		);
	}

	/// StepMania 5's own bundled reference `.sm` file (test.sm from the SDF docs).
	#[test]
	fn sm5_reference_test_file() {
		let bytes = test_file_read("sm/sm5_test.sm");
		let res = from_bytes(&bytes);

		// Metadata tags
		assert_eq!(
			res.get_first_value("TITLE").as_deref(),
			Some(b"test song" as &[u8])
		);
		assert_eq!(
			res.get_first_value("SUBTITLE").as_deref(),
			Some(b"a remix" as &[u8])
		);
		assert_eq!(
			res.get_first_value("ARTIST").as_deref(),
			Some(b"kurt angle!" as &[u8])
		);
		assert_eq!(
			res.get_first_value("CREDIT").as_deref(),
			Some(b"aj jelly" as &[u8])
		);
		assert_eq!(
			res.get_first_value("OFFSET").as_deref(),
			Some(b"-0.060" as &[u8])
		);
		assert_eq!(
			res.get_first_value("BPMS").as_deref(),
			Some(b"0.000=93.810,5.000=187.62" as &[u8])
		);

		// The file has 6 #NOTES entries (Beginner, Easy, Medium, Hard, Challenge, Edit).
		let notes = res.all_with_tag("NOTES");
		assert_eq!(notes.len(), 6, "expected 6 #NOTES entries");

		// Each #NOTES must have at least 6 params (steps_type, author, diff, level, groove, notedata).
		for note in &notes {
			assert!(
				note.values.len() >= 6,
				"#NOTES must have at least 6 params, got {}",
				note.values.len()
			);
		}

		// Difficulties in order
		let diffs: Vec<&str> = notes
			.iter()
			.map(|n| std::str::from_utf8(&n.values[2]).unwrap())
			.collect();
		assert_eq!(
			diffs,
			["Beginner", "Easy", "Medium", "Hard", "Challenge", "Edit"]
		);

		// Comments inside notedata (// measure N) must be stripped — check that
		// the Challenge chart's notedata doesn't contain "//" anywhere.
		let challenge_notedata = std::str::from_utf8(&notes[4].values[5]).unwrap();
		assert!(
			!challenge_notedata.contains("//"),
			"comments inside notedata should be stripped"
		);
	}

	/// Verify that the scarlet rose SM file parses correctly.
	#[test]
	fn scarlet_rose() {
		let bytes = test_file_read("sm/scarlet rose.sm");
		let res = from_bytes(&bytes);

		assert_eq!(
			res.get_first_value("TITLE").as_deref(),
			Some(b"Scarlet Rose" as &[u8])
		);
		assert_eq!(
			res.get_first_value("ARTIST").as_deref(),
			Some(b"Lily" as &[u8])
		);
		assert_eq!(
			res.get_first_value("CREDIT").as_deref(),
			Some(b"sorae" as &[u8])
		);

		// BPM is a single value on one line.
		assert_eq!(
			res.get_first_value("BPMS").as_deref(),
			Some(b"0.000=160.000" as &[u8])
		);

		// Two difficulties: Hard 13 and Challenge 14.
		let notes = res.all_with_tag("NOTES");
		assert_eq!(notes.len(), 2);
		assert_eq!(&*notes[0].values[0], b"dance-single");
		assert_eq!(&*notes[0].values[2], b"Hard");
		assert_eq!(&*notes[0].values[3], b"13");
		assert_eq!(&*notes[1].values[2], b"Challenge");
		assert_eq!(&*notes[1].values[3], b"14");
	}

	/// Verify that multi-line BPM lists (xsfull.sm) are captured in full.
	#[test]
	fn xsfull_multiline_bpms() {
		let bytes = test_file_read("sm/xsfull.sm");
		let res = from_bytes(&bytes);

		assert_eq!(
			res.get_first_value("TITLE").as_deref(),
			Some(b"[21] [156] XS Project Collection Full" as &[u8])
		);

		// BPMs span many lines and are joined into a single param value.
		let bpms = res.get_first_value("BPMS").expect("BPMS tag missing");
		// Must start with the first BPM entry.
		assert!(
			bpms.starts_with(b"0.000=149.000"),
			"BPMS should start with 0.000=149.000, got: {:?}",
			std::str::from_utf8(&bpms[..20.min(bpms.len())]).unwrap_or("?"),
		);
		// Must contain multiple BPM changes (comma-separated).
		assert!(
			bpms.contains(&b','),
			"multi-line BPMS should contain commas"
		);
	}

	/// Goin' Under — the song bundled with StepMania 5.
	#[test]
	fn goin_under() {
		let bytes = test_file_read("sm/goin_under.sm");
		let res = from_bytes(&bytes);

		assert!(
			res.get_first_value("TITLE").is_some(),
			"TITLE must be present"
		);
		assert!(
			res.get_first_value("ARTIST").is_some(),
			"ARTIST must be present"
		);

		// Must have at least one #NOTES entry.
		let notes = res.all_with_tag("NOTES");
		assert!(!notes.is_empty(), "expected at least one #NOTES entry");

		for note in &notes {
			assert!(
				note.values.len() >= 6,
				"#NOTES must have at least 6 params, got {}",
				note.values.len()
			);
		}
	}

	#[test]
	fn write_msd() {
		let res = serialize_msd_elements(vec![
			MsdElement {
				tag: Box::new(*b"TITLE"),
				values: vec![Box::new(*b"Song :: Title Here")],
			},
			MsdElement {
				tag: Box::new(*b"NOTES"),
				values: vec![
					Box::new(*b"dance-single"),
					Box::new(*b"Charter"),
					Box::new(*b"Beginner"),
					Box::new(*b"1"),
					Box::new(*b"0,0,0,0,0"),
					Box::new(
						*b"1000
0100
0010
0000",
					),
				],
			},
		]);

		let expected_out: Box<[u8]> = Box::new(
			*b"#TITLE:Song \\:\\: Title Here;
#NOTES:
     dance-single:
     Charter:
     Beginner:
     1:
     0,0,0,0,0:
1000
0100
0010
0000;
",
		);

		assert_eq!(res, expected_out);
	}
}
