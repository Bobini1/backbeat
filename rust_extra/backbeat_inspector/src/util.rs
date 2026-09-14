/// Trim, normalize separators, and drop empty paths.
pub(crate) fn path_string(raw: impl AsRef<str>) -> Option<String> {
	let s = raw.as_ref().trim().replace('\\', "/");
	if s.is_empty() { None } else { Some(s) }
}

/// Lossy UTF-8 from MSD / byte tags.
pub(crate) fn tag_string(raw: Option<&[u8]>) -> Option<String> {
	let bytes = raw?;
	path_string(String::from_utf8_lossy(bytes).as_ref())
}

/// Non-empty trimmed text (not a path — keeps original spacing collapsed only at ends).
pub(crate) fn text(raw: impl AsRef<str>) -> Option<String> {
	let s = raw.as_ref().trim();
	if s.is_empty() {
		None
	} else {
		Some(s.to_owned())
	}
}

pub(crate) fn text_opt(raw: Option<&str>) -> Option<String> {
	raw.and_then(text)
}
