use std::path::Path;

pub(crate) fn safe_extension(path: &Path) -> Option<&str> {
	if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
		return Some(ext);
	}
	let name = path.file_name()?.to_str()?;
	let rest = name.strip_prefix('.')?;
	if !rest.is_empty() && !rest.contains('.') {
		Some(rest)
	} else {
		None
	}
}
