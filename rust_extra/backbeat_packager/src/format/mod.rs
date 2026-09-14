use std::path::Path;

use backbeat_core::BackbeatFile;

use crate::error::PackageError;
use crate::fracturing::{self, FractureError, MeldError};
use crate::seen_cache::SeenCache;
use crate::util::safe_extension;

pub(crate) mod bms;
pub(crate) mod bmson;
pub(crate) mod dwi;
pub(crate) mod ksm;
pub(crate) mod sm;
pub mod sm_enrich;
pub(crate) mod ssc;

/// What formats the packager recognises and can specifically package.
///
/// If you're adding support for a format for _this_ packager, add it here.
///
/// If you're reading this and just want to package things for your own game,
/// consider writing your own script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Format {
	Bms,
	Bme,
	Bml,
	Pms,
	Sm,
	Bmson,
	Ksh,
	Kson,
	Ssc,
	Dwi,
}

// to add a new format, just fill out all the match branches below
impl Format {
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::Bms => "bms",
			Self::Bme => "bme",
			Self::Bml => "bml",
			Self::Pms => "pms",
			Self::Sm => "sm",
			Self::Bmson => "bmson",
			Self::Ksh => "ksh",
			Self::Kson => "kson",
			Self::Ssc => "ssc",
			Self::Dwi => "dwi",
		}
	}

	/// How many "../"s do we allow before the packager tells you you're being cheeky?
	///
	/// For SM, this is capped at 1, as there is never any good reason to path into other
	/// people's folders.
	///
	/// BMS rarely does it, but it's still done and bms files are immutable.
	///
	/// KSH does it _all the time_. Literally all the time.
	pub(crate) const fn max_allowed_up_pathing(self) -> usize {
		match self {
			Self::Bms
			| Self::Bme
			| Self::Bml
			| Self::Pms
			| Self::Bmson
			| Self::Ksh
			| Self::Kson => 5,
			Self::Sm | Self::Ssc | Self::Dwi => 1,
		}
	}

	/// Should this format be fractured? Fracturing means that multiple charts are stored
	/// in one file (think `.sm` files) vs one file. Backbeat strictly expects the `chart`
	/// content to be one playable "thing".
	pub(crate) const fn should_fracture(self) -> bool {
		match self {
			Self::Bms
			| Self::Bme
			| Self::Bml
			| Self::Pms
			| Self::Bmson
			| Self::Ksh
			| Self::Kson => false,
			Self::Sm | Self::Ssc | Self::Dwi => true,
		}
	}

	pub(crate) const fn can_meld(self) -> bool {
		match self {
			Self::Sm | Self::Ssc | Self::Dwi => true,
			Self::Bms
			| Self::Bme
			| Self::Bml
			| Self::Pms
			| Self::Bmson
			| Self::Ksh
			| Self::Kson => false,
		}
	}

	pub(crate) fn fracture_bytes(self, bytes: &[u8]) -> Result<Vec<Box<[u8]>>, FractureError> {
		match self {
			Self::Sm => fracturing::sm::fracture_bytes(bytes),
			Self::Ssc => fracturing::ssc::fracture_bytes(bytes),
			Self::Dwi => fracturing::dwi::fracture_bytes(bytes),
			Self::Bms
			| Self::Bme
			| Self::Bml
			| Self::Pms
			| Self::Bmson
			| Self::Ksh
			| Self::Kson => Err(FractureError::UnsupportedFormat(self.as_str().to_owned())),
		}
	}

	pub(crate) fn meld_bytes(self, inputs: &[Vec<u8>]) -> Result<Box<[u8]>, MeldError> {
		match self {
			Self::Sm => Ok(fracturing::sm::meld_bytes(inputs)?),
			Self::Ssc => Ok(fracturing::ssc::meld_bytes(inputs)?),
			Self::Dwi => Ok(fracturing::dwi::meld_bytes(inputs)?),
			Self::Bms
			| Self::Bme
			| Self::Bml
			| Self::Pms
			| Self::Bmson
			| Self::Ksh
			| Self::Kson => Err(MeldError::UnsupportedFormat(self.as_str().to_owned())),
		}
	}

	pub(crate) fn resolve_assets(
		self,
		bb: &mut BackbeatFile,
		path: &Path,
		cache: &SeenCache,
	) -> Result<(), PackageError> {
		match self {
			Self::Bms | Self::Bme | Self::Bml | Self::Pms => bms::resolve_assets(bb, path, cache)?,
			Self::Sm => sm::resolve_assets(bb, path, cache)?,
			Self::Ssc => ssc::resolve_assets(bb, path, cache)?,
			Self::Dwi => dwi::resolve_assets(bb, path, cache)?,
			Self::Bmson => bmson::resolve_assets(bb, path, cache)?,
			Self::Ksh => ksm::resolve_assets_ksh(bb, path, cache)?,
			Self::Kson => ksm::resolve_assets_kson(bb, path, cache)?,
		}
		Ok(())
	}

	pub(crate) fn from_extension(extension: &str) -> Option<Self> {
		match extension.to_ascii_lowercase().as_str() {
			"bms" => Some(Self::Bms),
			"bme" => Some(Self::Bme),
			"bml" => Some(Self::Bml),
			"pms" => Some(Self::Pms),
			"sm" => Some(Self::Sm),
			"bmson" => Some(Self::Bmson),
			"ksh" => Some(Self::Ksh),
			"kson" => Some(Self::Kson),
			"ssc" => Some(Self::Ssc),
			"dwi" => Some(Self::Dwi),
			_ => None,
		}
	}

	pub(crate) fn from_path(path: impl AsRef<Path>) -> Option<Self> {
		Self::from_extension(safe_extension(path.as_ref())?)
	}
}
