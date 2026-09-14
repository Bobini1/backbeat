use std::path::{Path, PathBuf};

use anyhow::Context;

pub(super) fn resolve_output_dir(
	source: &Path,
	output_dir: Option<&Path>,
) -> anyhow::Result<PathBuf> {
	let dir = match output_dir {
		Some(dir) => dir.to_path_buf(),
		None => source
			.parent()
			.filter(|parent| !parent.as_os_str().is_empty())
			.map(Path::to_path_buf)
			.unwrap_or_else(|| PathBuf::from(".")),
	};
	fs_err::create_dir_all(&dir)
		.with_context(|| format!("failed to create output directory {}", dir.display()))?;
	Ok(dir)
}
