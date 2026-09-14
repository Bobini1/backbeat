use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;

use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::util::safe_extension;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AssetPathError {
	#[error("This asset path is absolute, which is disallowed by backbeat.")]
	Absolute,

	#[error("This asset path contains NUL characters, which is illegal on all operating systems.")]
	NulChar,

	#[error(
		"The asset path {0} is disallowed as it will cause issues on some platforms. Paths must be unicode and must be representable on windows, linux and macOS."
	)]
	IllegalFilename(String),

	#[error("You cannot have paths that end in a slash.")]
	IsAFolder,

	#[error("This path is empty")]
	Empty,

	#[error("Asset path names cannot contain '\\' characters. Convert them to '/'.")]
	ContainsWindowsPaths,

	#[error("This asset path was not valid unicode text.")]
	NotUnicode,

	#[error("Asset paths cannot start or end with spaces")]
	UntrimmedWhitespace,
}

pub(crate) fn canonicalise_filepath(str: &str) -> String {
	// for my own fucking sanity, i'm going to truncate whitespace
	// from the filename
	str.trim_ascii().replace('\\', "/")
}

/// An asset path is a wrapper around a unicode string that denies certain
/// pathnames. It is the left hand side of the `assets` map in a backbeat file.
///
/// Take a look at [`AssetPathError`] to see what's disallowed.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, DeserializeFromStr, SerializeDisplay, PartialOrd, Ord,
)]
pub struct AssetPath(String);

impl AssetPath {
	/// Perform no canonicalisation or mutations, just interpet the unicode string.
	pub fn new(unicode: &str) -> Result<Self, AssetPathError> {
		if unicode.trim().len() != unicode.len() {
			return Err(AssetPathError::UntrimmedWhitespace);
		}

		if unicode.contains('\\') {
			return Err(AssetPathError::ContainsWindowsPaths);
		}

		if unicode.is_empty() {
			return Err(AssetPathError::Empty);
		}

		if unicode.contains('\0') {
			return Err(AssetPathError::NulChar);
		}

		// this is the unix check for absolute paths
		if unicode.starts_with('/') {
			return Err(AssetPathError::Absolute);
		}

		if unicode.ends_with('/') {
			return Err(AssetPathError::IsAFolder);
		}

		// this is the windows check for things like X:/
		if unicode
			.chars()
			.next()
			.is_some_and(|character| character.is_ascii_alphabetic())
			&& unicode
				.get(1..)
				.is_some_and(|suffix| suffix.starts_with(":/"))
		{
			return Err(AssetPathError::Absolute);
		}

		let unicode = unicode.to_owned();

		if is_illegal_filename(&unicode) {
			return Err(AssetPathError::IllegalFilename(unicode));
		}

		// OK, the path is safe.
		Ok(Self(unicode))
	}

	/// Normalise the path coming in and parse it. This is for ingesting new paths.
	pub fn from_path(p: impl AsRef<Path>) -> Result<Self, AssetPathError> {
		let unicode = p.as_ref().to_str().ok_or(AssetPathError::NotUnicode)?;

		let canonicalised = canonicalise_filepath(unicode);
		let canonical = canonicalised.trim_start_matches('/');
		if canonical.len() != canonicalised.len() {
			tracing::warn!(
				original_path = %unicode,
				canonical_path = %canonical,
				"asset path starts with path separators; treating it as relative"
			);
		}

		Self::new(canonical)
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn as_path(&self) -> &Path {
		Path::new(&self.0)
	}

	/// Get the extension for this filename.
	pub fn extension(&self) -> Option<&str> {
		safe_extension(self.as_path())
	}

	/// Return this path with its final component's extension replaced.
	pub fn with_extension(&self, extension: &str) -> Result<Self, AssetPathError> {
		let filename_start = self.0.rfind('/').map_or(0, |index| index + 1);
		let filename = &self.0[filename_start..];
		let stem_end = match filename.rfind('.') {
			Some(index) if index != 0 => filename_start + index,
			_ => self.0.len(),
		};

		let mut path = self.0[..stem_end].to_owned();
		if !extension.is_empty() {
			path.push('.');
			path.push_str(extension);
		}

		Self::from_path(&path)
	}
}

impl AsRef<Path> for AssetPath {
	fn as_ref(&self) -> &Path {
		self.as_path()
	}
}

impl Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

impl FromStr for AssetPath {
	type Err = AssetPathError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::new(s)
	}
}

// deliberately pointlessc
const fn is_unsafe_on_linux(_unicode: &str) -> bool {
	false
}

// deliberately pointlessc
const fn is_unsafe_on_macos(_unicode: &str) -> bool {
	false
}

fn is_unsafe_on_windows(unicode: &str) -> bool {
	if unicode.chars().any(|character| {
		character < '\u{20}' || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\')
	}) {
		return true;
	}

	unicode.split('/').any(is_reserved_windows_component)
}

fn is_reserved_windows_component(component: &str) -> bool {
	let component = component.trim_end_matches([' ', '.']);
	let stem = component.split('.').next().unwrap_or_default();
	let stem = stem.trim_end_matches([' ', '.']);

	["CON", "PRN", "AUX", "NUL", "CLOCK$"]
		.iter()
		.any(|name| stem.eq_ignore_ascii_case(name))
		|| is_numbered_windows_device(stem, "COM")
		|| is_numbered_windows_device(stem, "LPT")
}

fn is_numbered_windows_device(stem: &str, prefix: &str) -> bool {
	let Some(candidate_prefix) = stem.get(..prefix.len()) else {
		return false;
	};
	if !candidate_prefix.eq_ignore_ascii_case(prefix) {
		return false;
	}

	let suffix = &stem[prefix.len()..];
	if suffix.len() == 1 && matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") {
		return true;
	}

	matches!(suffix, "¹" | "²" | "³")
}

pub(crate) fn is_illegal_filename(unicode: &str) -> bool {
	if unicode == ".." || unicode == "." || unicode.ends_with("/..") || unicode.ends_with("/.") {
		return true;
	}

	is_unsafe_on_linux(unicode) || is_unsafe_on_macos(unicode) || is_unsafe_on_windows(unicode)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn asset_path_with_extension_replaces_only_the_final_component_extension() {
		let path = AssetPath::from_path("folder.v1/song.old.ogg").unwrap();

		assert_eq!(
			path.with_extension("wav").unwrap().as_str(),
			"folder.v1/song.old.wav"
		);
	}

	#[test]
	fn canonicalise_asset_path_removes_leading_path_separators() {
		let path = AssetPath::from_path(r"\\//miss/foo.bmp").unwrap();
		assert_eq!(path.as_str(), "miss/foo.bmp");
	}

	#[test]
	fn asset_path_with_backslash_reports_windows_path_error() {
		let error = AssetPath::new(r"folder\file.ogg").unwrap_err();

		assert!(matches!(error, AssetPathError::ContainsWindowsPaths));
	}

	#[test]
	fn detects_windows_reserved_device_names() {
		for path in [
			"CON",
			"con.txt",
			"CON .txt",
			"folder/PRN ",
			"AUX.",
			"NUL.anything",
			"CLOCK$",
			"COM1.wav",
			"lpt9.png",
			"COM¹.wav",
		] {
			assert!(is_unsafe_on_windows(path), "{path:?}");
		}
	}

	#[test]
	fn allows_non_reserved_windows_names() {
		for path in [
			"CONSOLE",
			"COM0.wav",
			"COM10.wav",
			"LPT10.png",
			"folder/normal.txt",
		] {
			assert!(!is_unsafe_on_windows(path), "{path:?}");
		}
	}

	#[test]
	fn detects_windows_illegal_characters() {
		for path in [
			"less<than",
			"greater>than",
			"colon:name",
			"quote\"name",
			"pipe|name",
			"question?name",
			"star*name",
			"back\\slash",
			"control\x01name",
		] {
			assert!(is_unsafe_on_windows(path), "{path:?}");
		}
	}

	#[test]
	fn allows_slashes_as_path_separators_on_windows() {
		assert!(!is_unsafe_on_windows("folder/name.txt"));
	}
}
