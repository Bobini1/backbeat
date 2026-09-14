pub(crate) const MAX_FOLDER_NAME_CHARS: usize = 100;

pub(crate) fn sanitize_folder_name(name: &str, fallback: &str) -> String {
	let mut sanitized: String = name
		.chars()
		.map(|character| {
			if character.is_control()
				|| matches!(
					character,
					'/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
				) {
				'-'
			} else {
				character
			}
		})
		.collect();

	sanitized = sanitized.trim_end_matches([' ', '.']).to_owned();
	if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
		return fallback.to_owned();
	}

	let basename = sanitized.split('.').next().unwrap_or_default();
	if matches!(
		basename.to_ascii_uppercase().as_str(),
		"CON"
			| "PRN" | "AUX"
			| "NUL" | "COM1"
			| "COM2" | "COM3"
			| "COM4" | "COM5"
			| "COM6" | "COM7"
			| "COM8" | "COM9"
			| "LPT1" | "LPT2"
			| "LPT3" | "LPT4"
			| "LPT5" | "LPT6"
			| "LPT7" | "LPT8"
			| "LPT9"
	) {
		sanitized.push('-');
	}

	sanitized
}

pub(crate) fn truncate_folder_name(name: &str, limit: usize) -> String {
	name.chars().take(limit).collect()
}
