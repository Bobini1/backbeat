//! Errors from base packaging and format-specific asset resolution.

use backbeat_inspector::InspectError;

use crate::format::{bms, bmson, dwi, ksm, sm, ssc};

/// Error building a [`backbeat_core::BackbeatFile`]
#[derive(Debug, thiserror::Error)]
pub enum FromFileError {
	#[error("could not read chart file: {0}")]
	Io(#[from] std::io::Error),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] backbeat_core::BackbeatFileError),

	#[error("StepMania asset {path:?} traverses more than one parent directory")]
	StepmaniaAssetPathTooDeep { path: String },

	#[error("asset {path:?} traverses more than {max_depth} parent directories")]
	AssetPathTooDeep { path: String, max_depth: usize },

	#[error("invalid chart filename: {0}")]
	Filename(#[from] backbeat_core::ChartFilenameError),

	#[error("Failed to inspect the chart: {0}")]
	Inspect(#[from] InspectError),

	#[error("invalid asset path: {0}")]
	AssetPath(#[from] backbeat_core::AssetPathError),

	#[error(
		"we don't know what filetype this is {0}, either write your own packager or add support upstream."
	)]
	UnrecognisedFiletype(String),
}

/// Error returned by [`crate::package`], [`crate::package_with_cache`], and [`crate::fracture`].
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error(transparent)]
	BbZip(#[from] backbeat_core::BbZipWriteError),

	#[error(transparent)]
	FromFile(#[from] FromFileError),

	#[error(transparent)]
	Bms(#[from] bms::BmsPackageError),

	#[error(transparent)]
	Sm(#[from] sm::SmPackageError),

	#[error(transparent)]
	Dwi(#[from] dwi::DwiPackageError),

	#[error(transparent)]
	Ssc(#[from] ssc::SscPackageError),

	#[error(transparent)]
	Bmson(#[from] bmson::BmsonPackageError),

	#[error(transparent)]
	Ksm(#[from] ksm::KsmPackageError),

	#[error(transparent)]
	Fracture(#[from] crate::fracturing::FractureError),
}
