/// Controls how a BMS parser resolves `#RANDOM` directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsRandomStrategy {
	/// Always select branch `1`.
	AlwaysFirstBranch,
}

impl BmsRandomStrategy {
	/// Preprocess decoded BMS text according to this random strategy.
	///
	/// impl cribbed from openlr2
	pub fn preprocess(&self, text: &str) -> String {
		let mut output = String::with_capacity(text.len());
		let mut selected = -1;
		let mut if_on = true;

		for line in text.split_inclusive(['\n', '\r']) {
			let Some(directive) = RandomDirective::from_line(line) else {
				if if_on {
					output.push_str(line);
				}
				continue;
			};

			match directive {
				RandomDirective::Random(upper_bound) => {
					selected = self.select(upper_bound);
					if_on = false;
				}
				RandomDirective::If(branch) => {
					if_on = branch == selected;
				}
				RandomDirective::EndIf => {
					if_on = true;
				}
			}
		}

		output
	}

	fn select(&self, _upper_bound: i32) -> i32 {
		match self {
			Self::AlwaysFirstBranch => 1,
		}
	}
}

enum RandomDirective {
	Random(i32),
	If(i32),
	EndIf,
}

impl RandomDirective {
	fn from_line(line: &str) -> Option<Self> {
		let line = line.trim_ascii();

		if starts_with_ignore_ascii_case(line, "#RANDOM") {
			Some(Self::Random(parse_integer(argument(line, 7))))
		} else if starts_with_ignore_ascii_case(line, "#ENDIF") {
			Some(Self::EndIf)
		} else if starts_with_ignore_ascii_case(line, "#IF") {
			Some(Self::If(parse_integer(argument(line, 3))))
		} else {
			None
		}
	}
}

fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
	text.get(..prefix.len())
		.is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn argument(line: &str, directive_len: usize) -> &str {
	line.get(directive_len + 1..).unwrap_or_default()
}

fn parse_integer(text: &str) -> i32 {
	let mut bytes = text.trim_ascii_start().as_bytes();
	let negative = bytes.starts_with(b"-");
	if negative || bytes.starts_with(b"+") {
		bytes = &bytes[1..];
	}

	let mut value = 0_i64;
	for digit in bytes.iter().take_while(|digit| digit.is_ascii_digit()) {
		value = value
			.saturating_mul(10)
			.saturating_add(i64::from(*digit - b'0'));
	}

	if negative {
		value = -value;
	}

	value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
	use super::*;

	fn contains_line(output: &str, expected: &str) -> bool {
		output.split(['\n', '\r']).any(|line| line == expected)
	}

	#[test]
	fn always_first_branch() {
		let text = "#TITLE Test\n\
			#RANDOM 2\n\
			#IF 1\n\
			#00111:0100\n\
			#ENDIF\n\
			#IF 2\n\
			#00112:0100\n\
			#ENDIF\n\
			#ENDRANDOM\n";

		let output = BmsRandomStrategy::AlwaysFirstBranch.preprocess(text);

		assert!(contains_line(&output, "#00111:0100"));
		assert!(!contains_line(&output, "#00112:0100"));
		assert!(!contains_line(&output, "#RANDOM 2"));
	}

	#[test]
	fn nested_random_overwrites_outer_state() {
		let text = "#RANDOM 2\n\
			#IF 2\n\
			#RANDOM 1\n\
			#IF 1\n\
			#TITLE Leak\n\
			#ENDIF\n\
			#ENDRANDOM\n\
			#ENDIF\n\
			#IF 1\n\
			#TITLE Keep\n\
			#ENDIF\n\
			#ENDRANDOM\n";

		let output = BmsRandomStrategy::AlwaysFirstBranch.preprocess(text);

		assert!(contains_line(&output, "#TITLE Leak"));
		assert!(contains_line(&output, "#TITLE Keep"));
	}

	#[test]
	fn random_disables_lines_until_an_if() {
		let text = "#RANDOM 2\n\
			#TITLE Discard\n\
			#IF 1\n\
			#TITLE Keep\n\
			#ENDIF\n\
			#ENDRANDOM\n";

		let output = BmsRandomStrategy::AlwaysFirstBranch.preprocess(text);

		assert!(!contains_line(&output, "#TITLE Discard"));
		assert!(contains_line(&output, "#TITLE Keep"));
	}

	#[test]
	fn zero_and_negative_random_bounds_select_first_branch() {
		for bound in ["0", "-1"] {
			let text = format!(
				"#RANDOM {bound}\n\
				 #IF 1\n\
				 #TITLE Keep\n\
				 #ENDIF\n\
				 #ENDRANDOM\n"
			);

			let output = BmsRandomStrategy::AlwaysFirstBranch.preprocess(&text);

			assert!(contains_line(&output, "#TITLE Keep"));
		}
	}

	#[test]
	fn integers_are_parsed_like_atol() {
		assert_eq!(parse_integer("  -12xyz"), -12);
		assert_eq!(parse_integer("1.5"), 1);
		assert_eq!(parse_integer("xyz"), 0);
		assert_eq!(parse_integer(""), 0);
		assert_eq!(parse_integer("999999999999999999999"), i32::MAX);
	}

	#[test]
	fn directives_are_prefix_matched() {
		assert!(matches!(
			RandomDirective::from_line("#RANDOMNESS"),
			Some(RandomDirective::Random(0))
		));
		assert!(matches!(
			RandomDirective::from_line("#IF+1"),
			Some(RandomDirective::If(1))
		));
		assert!(matches!(
			RandomDirective::from_line("#ENDIF_SUFFIX"),
			Some(RandomDirective::EndIf)
		));
		assert!(RandomDirective::from_line("#ENDRANDOM").is_none());
	}
}
