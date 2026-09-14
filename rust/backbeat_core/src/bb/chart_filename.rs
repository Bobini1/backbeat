use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;

use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::chart_id::RecognisedChartIdAlgorithm;
use crate::util::safe_extension;

use crate::asset_path::{canonicalise_filepath, is_illegal_filename};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChartFilenameError {
	#[error("Filenames cannot contain '/' characters.")]
	ContainsSlashes,

	#[error("This filename contains NUL characters, which is illegal on all operating systems.")]
	NulChar,

	#[error(
		"The filename {0} is disallowed as it will cause issues on some platforms. Paths must be unicode and must be representable on windows, linux and macOS."
	)]
	IllegalFilename(String),

	#[error("This filename is empty")]
	Empty,

	#[error("Chart filenames cannot start or end with spaces")]
	UntrimmedWhitespace,

	#[error("Chart filenames must be utf-8.")]
	NotUnicode,

	#[error(
		"You can't have a `.bb` file that bundles another '.bb' file. That's unbelievably confusing."
	)]
	HasBbExtension,
}

/// A safe wrapper around a unicode filename - this is a single file component,
/// no pathing allowed. Some filenames (e.g. ".." and "CON" are banned)
///
/// For a variant that allows pathing, use [`crate::AssetPath`].
#[derive(
	Debug, Clone, Hash, PartialEq, Eq, DeserializeFromStr, SerializeDisplay, PartialOrd, Ord,
)]
pub struct ChartFilename(String);

impl ChartFilename {
	/// Given a chart path, canonicalise (normalise) it (replace \ and trim whitespace) then
	/// run it through the usual checks.
	///
	/// If you are parsing user input, use `::new()` instead.
	pub fn from_path(p: impl AsRef<Path>) -> Result<Self, ChartFilenameError> {
		let s = p.as_ref().to_str().ok_or(ChartFilenameError::NotUnicode)?;

		Self::new(&canonicalise_filepath(s))
	}

	/// Create a new [`ChartFilename`], upholding the filename restrictions.
	pub fn new(unicode: &str) -> Result<Self, ChartFilenameError> {
		if unicode.trim().len() != unicode.len() {
			return Err(ChartFilenameError::UntrimmedWhitespace);
		}

		if unicode.is_empty() {
			return Err(ChartFilenameError::Empty);
		}

		if unicode.contains('\0') {
			return Err(ChartFilenameError::NulChar);
		}

		if unicode.contains('/') {
			return Err(ChartFilenameError::ContainsSlashes);
		}

		if Path::new(unicode)
			.extension()
			.is_some_and(|ext| ext.eq_ignore_ascii_case("bb"))
		{
			return Err(ChartFilenameError::HasBbExtension);
		}

		let unicode = unicode.to_owned();
		if is_illegal_filename(&unicode) {
			return Err(ChartFilenameError::IllegalFilename(unicode));
		}

		Ok(Self(unicode))
	}

	/// Get the extension for this filename.
	pub fn extension(&self) -> Option<&str> {
		safe_extension(self.as_path())
	}

	/// Check whether this chart has a file extension with this case, ascii insensitively.
	pub fn ends_with_case_insensitive(&self, ext: &str) -> bool {
		self.as_str()
			.to_ascii_lowercase()
			.ends_with(&ext.to_ascii_lowercase())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn as_path(&self) -> &Path {
		Path::new(&self.0)
	}

	/// What additional chart ID algorithms to calculate for this file type?
	pub fn id_algorithms(&self) -> &'static [RecognisedChartIdAlgorithm] {
		// n.b. This isn't "what algorithms _can_ be calculated" for a given file, otherwise
		// we'd unconditionally calculate md5 for all files. Instead, this is what algorithms _should_
		// be calculated for a given file.
		//
		// This feature used to have a lot more going on, but there's just _far_ too many flaws
		// in implementing popular rhythm game "custom chart ids".
		//
		// For example:
		// - KSM's "IR Hash" will call `kson_to_ksh` before doing calculations, meaning you have to link a whole CPP library in,
		//   and that conversion isn't even deterministic anyway, so the whole thing is fucked
		// - Etterna's ChartKey is broken and only checks the order of notes in a chart + bpms, so changing the spacing between
		//   notes results in the same hash. Also, it's so tightly wound into stepmania logic that reimplementing it is impossible.
		//   Look at how it handles warps, and feel free to set "GPT-7 MEGA MODE" on it in the future to try and get it into a library,
		//   or don't, because it's a shit algorithm.
		// - Groovestats v3 is broken; it quantises notes incorrectly (the dividing algorithm is wrong) and then _truncates the hash_
		//   to such a small amount of characters that hash collisions are practically guaranteed by the pigeonhole principle.
		//   I think GSv3 truncates to 10 hex characters, meaning there are only 1 trillion possible chart IDs. Birthday problem
		//   and all that, and I think you will legitimately get _incidental_ collisions with this algorithm. Obviously, you can
		//   create deliberate collisions with no effort.
		//
		// So all three of those algorithms got removed. And then I just removed sha1 aswell because why bother? USC "uses" it
		// but USC has no tables to speak of so there's not anything to preserve. USC really uses "path on your filesystem" like
		// stepmania. KSM doesn't even have databases, so who cares.
		//
		// We still support md5 because it's used _widely_ across BMS and not having support for it would be a huge loss for no gain.
		// That's the only reason I've kept this feature around. In the future, someone will define a good "NotesID" algorithm.
		//
		// By the way, if you're writing your own rhythm game (or integrating backbeat), or just want to experiment, you can
		// add and define your own chart algorithms here and just _ship your version of the backbeat library with the game_.
		// I don't hold you hostage, and everything will work so long as your version of the backbeat library is capable
		// of calculating the chart IDs you want for the file formats you support. The store is global and works fine with
		// different versions of the library on your PC.
		match self.extension().map(|e| e.to_lowercase()).as_deref() {
			Some("bms" | "bme" | "bml" | "pms" | "bmson") => &[RecognisedChartIdAlgorithm::Md5],
			Some(_) | None => &[],
		}
	}
}

impl AsRef<Path> for ChartFilename {
	fn as_ref(&self) -> &Path {
		self.as_path()
	}
}

impl Display for ChartFilename {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

impl FromStr for ChartFilename {
	type Err = ChartFilenameError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::new(s)
	}
}
