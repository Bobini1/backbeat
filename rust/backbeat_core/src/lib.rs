#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

#[cfg(test)]
#[macro_use]
mod fixtures;

#[cfg(feature = "fs_err")]
pub(crate) use fs_err as fs;
#[cfg(feature = "fs_err")]
pub(crate) use fs_err::File;

#[cfg(not(feature = "fs_err"))]
pub(crate) use std::fs;
#[cfg(not(feature = "fs_err"))]
pub(crate) use std::fs::File;

pub mod asset_id;
pub mod asset_path;
pub mod bb;
pub mod bbzip;
pub mod chart_id;
pub mod collections;
pub mod precombined_assets;
pub mod sha256;
#[cfg(feature = "sqlx")]
mod sqlx;
mod util;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use self::asset_id::AssetId;
pub use self::asset_path::{AssetPath, AssetPathError};
pub use self::bb::{
	Assets, BackbeatFile, BackbeatFileError, BundleId, ChartData, ChartDesc, ChartFilename,
	ChartFilenameError, CombinedAssetsId,
};
pub use self::bbzip::{BbZipEntry, BbZipReadError, BbZipReader, BbZipWriteError, BbZipWriter};
pub use self::chart_id::{ChartId, CustomIdAlgorithm, CustomIdAlgorithmError, IdAlgorithm};
pub use self::collections::{
	CollectionHeader, CollectionKind, Course, CourseChart, CourseChartTags, CourseTags, LevelTags,
	Pack, PackBundle, PackBundleTags, PackTags, Table, TableChart, TableChartTags, TableFolder,
	TableFolderTags, TableLevel, TableTags, ValidGamemodeIdentifier, ValidGamemodeIdentifierError,
};
pub use self::sha256::{ParseSha256Error, Sha256, Sha256Digest};
