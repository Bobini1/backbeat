//! Shared formatting utilities for human-readable output.
//!
//! The main types are the free functions [`count`] and [`bytes`] for number
//! formatting, and [`Table`] for two-column aligned key/value output.

use unicode_width::UnicodeWidthChar;

/// Format a number with thousands separators: `1234567` → `"1,234,567"`.
pub fn count(n: u64) -> String {
	let s = n.to_string();
	let mut out = String::with_capacity(s.len() + s.len() / 3);
	for (i, ch) in s.chars().rev().enumerate() {
		if i > 0 && i % 3 == 0 {
			out.push(',');
		}
		out.push(ch);
	}
	out.chars().rev().collect()
}

/// Format a byte count in the most appropriate unit: `"123 B"`, `"1.5 MB"`, etc.
pub fn bytes(n: u64) -> String {
	const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
	let mut value = n as f64;
	let mut unit = UNITS[0];
	for &u in &UNITS[1..] {
		if value < 1024.0 {
			break;
		}
		value /= 1024.0;
		unit = u;
	}
	if unit == "B" {
		format!("{n} B")
	} else {
		format!("{value:.1} {unit}")
	}
}

// ── Table ─────────────────────────────────────────────────────────────────────

/// A two-column key/value table that auto-computes column widths from its
/// content before printing, so no magic numbers are needed at the call site.
///
/// ```text
/// Charts:       12,345
///
/// Packs:         1,234
/// Scales:            0
///
/// Assets:       98,765
///   Filesystem: 95,000
///   Inline:      3,765
///
/// Storage:
///   Assets:    14.2 GB
///   Database: 890.1 MB
///   Total:     15.0 GB
/// ```
pub struct Table {
	rows: Vec<TableRow>,
}

enum TableRow {
	/// A data row: optional indent, a label, and a value.
	/// `right_align` controls whether the value column is right- or left-aligned.
	Data {
		indent: usize,
		label: String,
		value: String,
		right_align: bool,
	},
	/// A section header with no value (printed on its own line).
	Section(String),
	/// A blank separator line.
	Blank,
}

impl Table {
	pub fn new() -> Self {
		Self { rows: vec![] }
	}

	/// Top-level label + **right-aligned** value (for counts, byte sizes).
	pub fn row(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
		self.rows.push(TableRow::Data {
			indent: 0,
			label: label.into(),
			value: value.into(),
			right_align: true,
		});
		self
	}

	/// Indented (2-space) label + **right-aligned** value.
	pub fn sub(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
		self.rows.push(TableRow::Data {
			indent: 2,
			label: label.into(),
			value: value.into(),
			right_align: true,
		});
		self
	}

	/// Top-level label + **left-aligned** value (for paths, strings).
	pub fn kv(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
		self.rows.push(TableRow::Data {
			indent: 0,
			label: label.into(),
			value: value.into(),
			right_align: false,
		});
		self
	}

	/// Indented (2-space) label + **left-aligned** value.
	pub fn kv_sub(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
		self.rows.push(TableRow::Data {
			indent: 2,
			label: label.into(),
			value: value.into(),
			right_align: false,
		});
		self
	}

	/// A section header line printed without a value column.
	pub fn section(&mut self, label: impl Into<String>) -> &mut Self {
		self.rows.push(TableRow::Section(label.into()));
		self
	}

	/// A blank separator line.
	pub fn blank(&mut self) -> &mut Self {
		self.rows.push(TableRow::Blank);
		self
	}

	/// Print the table to stdout.
	///
	/// Column widths are derived from the actual content, so every value
	/// aligns to the same position regardless of label length.
	pub fn print(&self) {
		print!("{}", self.rendered());
	}

	fn rendered(&self) -> String {
		let mut output = String::new();
		// Max total width of the label column (indent + label chars).
		let label_col = self
			.rows
			.iter()
			.filter_map(|r| match r {
				TableRow::Data { indent, label, .. } => Some(indent + label.len()),
				_ => None,
			})
			.max()
			.unwrap_or(0);

		// Max width of the value column — only relevant for right-aligned rows,
		// where we pad shorter values to keep columns flush.
		let value_col = self
			.rows
			.iter()
			.filter_map(|r| match r {
				TableRow::Data {
					value,
					right_align: true,
					..
				} => Some(visible_width(value)),
				_ => None,
			})
			.max()
			.unwrap_or(0);

		for row in &self.rows {
			match row {
				TableRow::Data {
					indent,
					label,
					value,
					right_align,
				} => {
					let full = format!("{:indent$}{label}", "");
					if *right_align {
						let visible = visible_width(value);
						let padding = value_col.saturating_sub(visible);
						output.push_str(&format!(
							"{full:<label_col$}  {}{value}\n",
							" ".repeat(padding)
						));
					} else {
						output.push_str(&format!("{full:<label_col$}  {value}\n"));
					}
				}
				TableRow::Section(label) => {
					output.push_str(label);
					output.push('\n');
				}
				TableRow::Blank => output.push('\n'),
			}
		}

		output
	}
}

fn visible_width(input: &str) -> usize {
	let mut width = 0;
	let mut in_csi = false;
	for ch in input.chars() {
		match (in_csi, ch) {
			(true, '[') => {}                              // CSI introducer
			(true, '\u{40}'..='\u{7e}') => in_csi = false, // CSI final byte
			(true, _) => {}
			(false, '\x1b') => in_csi = true,
			(false, _) => width += ch.width().unwrap_or(0),
		}
	}
	width
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn count_small() {
		assert_eq!(count(0), "0");
		assert_eq!(count(999), "999");
		assert_eq!(count(1000), "1,000");
		assert_eq!(count(1_234_567), "1,234,567");
	}

	#[test]
	fn bytes_units() {
		assert_eq!(bytes(0), "0 B");
		assert_eq!(bytes(512), "512 B");
		assert_eq!(bytes(1024), "1.0 KiB");
		assert_eq!(bytes(1024 * 1024), "1.0 MiB");
		assert_eq!(bytes(1536 * 1024), "1.5 MiB");
	}
}
