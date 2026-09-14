use std::ffi::CStr;

use backbeat_core::precombined_assets::PrecombinedAssetsError;
use backbeat_core::{BackbeatFileError, BbZipReadError, BbZipWriteError};
use backbeat_sdk::{CollectionClientError, StoreError};
use backbeat_server_client::RemoteError;
use backbeat_store_config::ConfigError;

pub type bkb_error_code = i32;

pub const BKB_OK: bkb_error_code = 0;
pub const BKB_ERR_DB: bkb_error_code = 1;
pub const BKB_ERR_CORRUPT: bkb_error_code = 2;
pub const BKB_ERR_MIGRATE: bkb_error_code = 3;
pub const BKB_ERR_IO: bkb_error_code = 4;
pub const BKB_ERR_CONFIG: bkb_error_code = 5;
pub const BKB_ERR_JSON: bkb_error_code = 6;
pub const BKB_ERR_INVALID_COLLECTION_HEADER: bkb_error_code = 7;
pub const BKB_ERR_INVALID_BUNDLE: bkb_error_code = 8;
pub const BKB_ERR_TOO_LARGE: bkb_error_code = 9;
pub const BKB_ERR_ZIP: bkb_error_code = 10;
pub const BKB_ERR_NESTED_BBZIP: bkb_error_code = 11;
pub const BKB_ERR_PARSE: bkb_error_code = 12;
pub const BKB_ERR_NOT_FOUND: bkb_error_code = 13;
pub const BKB_ERR_HASH_MISMATCH: bkb_error_code = 14;
pub const BKB_ERR_INVALID_PRECOMBINED_ASSETS: bkb_error_code = 15;
pub const BKB_ERR_INVALID_ASSET_PATH: bkb_error_code = 16;
pub const BKB_ERR_ID_MISMATCH: bkb_error_code = 17;
pub const BKB_ERR_NO_SERVERS: bkb_error_code = 18;
pub const BKB_ERR_INVALID_URL: bkb_error_code = 19;
pub const BKB_ERR_NETWORK: bkb_error_code = 20;
pub const BKB_ERR_UNKNOWN_ALGORITHM: bkb_error_code = 21;
pub const BKB_ERR_PERMISSION_DENIED: bkb_error_code = 22;
pub const BKB_ERR_CANCELLED: bkb_error_code = 23;
pub const BKB_ERR_REMOTE_BAD_STATUS_CODE: bkb_error_code = 24;
pub const BKB_ERR_DOWNLOAD_TASK_FAILED: bkb_error_code = 25;

pub const BKB_ERR_NULL_ARG: bkb_error_code = 1000;
pub const BKB_ERR_PANIC: bkb_error_code = 1001;
pub const BKB_ERR_INVALID_STRING: bkb_error_code = 1002;
pub const BKB_ERR_INCOMPATIBLE_SQLITE: bkb_error_code = 1004;

pub(crate) fn backbeat_file_error_code(err: &BackbeatFileError) -> bkb_error_code {
	match err {
		BackbeatFileError::Json(_) => BKB_ERR_JSON,
		BackbeatFileError::ChartNotBase64
		| BackbeatFileError::ChartNotGzip(_)
		| BackbeatFileError::DescTooLarge => BKB_ERR_INVALID_BUNDLE,
		BackbeatFileError::ChartTooLarge { .. } => BKB_ERR_TOO_LARGE,
		BackbeatFileError::Io(_) => BKB_ERR_IO,
	}
}

pub(crate) fn run_backbeat_file<T>(
	op: impl FnOnce() -> Result<T, BackbeatFileError>,
) -> Result<T, bkb_error_code> {
	op().map_err(|err| backbeat_file_error_code(&err))
}

pub(crate) fn store_error_code(err: &StoreError) -> bkb_error_code {
	match err {
		StoreError::Db(_) => BKB_ERR_DB,
		StoreError::Corrupt(_) => BKB_ERR_CORRUPT,
		StoreError::Migrate(_) => BKB_ERR_MIGRATE,
		StoreError::Io(_) => BKB_ERR_IO,
		StoreError::Config(err) => match err {
			ConfigError::Io(_) => BKB_ERR_IO,
			ConfigError::Toml(_) | ConfigError::TomlSer(_) => BKB_ERR_CONFIG,
		},
		StoreError::Json(_) => BKB_ERR_JSON,
		StoreError::InvalidCollectionHeader { .. } => BKB_ERR_INVALID_COLLECTION_HEADER,
		StoreError::CollectionClient(err) => match err {
			CollectionClientError::Request(_) => BKB_ERR_NETWORK,
			CollectionClientError::HttpStatus { .. } => BKB_ERR_REMOTE_BAD_STATUS_CODE,
		},
		StoreError::Bundle(err) => backbeat_file_error_code(err),
		StoreError::BbZip(err) => match err {
			BbZipReadError::Zip(_) => BKB_ERR_ZIP,
			BbZipReadError::Io(_) => BKB_ERR_IO,
			BbZipReadError::InvalidBackbeatEntry { source, .. } => backbeat_file_error_code(source),
			BbZipReadError::NestedZip => BKB_ERR_NESTED_BBZIP,
		},
		StoreError::BbZipWrite(err) => match err {
			BbZipWriteError::Zip(_) => BKB_ERR_ZIP,
			BbZipWriteError::Io(_) => BKB_ERR_IO,
			BbZipWriteError::NestedZip => BKB_ERR_NESTED_BBZIP,
		},
		StoreError::Parse(_) => BKB_ERR_PARSE,
		StoreError::NotFound(_) => BKB_ERR_NOT_FOUND,
		StoreError::HashMismatch => BKB_ERR_HASH_MISMATCH,
		StoreError::PrecombinedAssets(err) => match err {
			PrecombinedAssetsError::Io(_) => BKB_ERR_IO,
			PrecombinedAssetsError::NonFileEntry
			| PrecombinedAssetsError::NonUtf8Path
			| PrecombinedAssetsError::UnexpectedPath(_)
			| PrecombinedAssetsError::DuplicatePath(_)
			| PrecombinedAssetsError::MissingPaths(_)
			| PrecombinedAssetsError::AssetNotFullyRead(_) => BKB_ERR_INVALID_PRECOMBINED_ASSETS,
			PrecombinedAssetsError::InvalidAssetPath { .. } => BKB_ERR_INVALID_ASSET_PATH,
			PrecombinedAssetsError::AssetHashMismatch { .. } => BKB_ERR_HASH_MISMATCH,
		},
		StoreError::BundleIdMismatch { .. } | StoreError::ChartIdMismatch { .. } => {
			BKB_ERR_ID_MISMATCH
		}
		StoreError::Remote(err) => match err {
			RemoteError::NoServers => BKB_ERR_NO_SERVERS,
			RemoteError::InvalidUrl(_) => BKB_ERR_INVALID_URL,
			RemoteError::Network(_) => BKB_ERR_NETWORK,
			RemoteError::Json(_) => BKB_ERR_JSON,
			RemoteError::NotFound => BKB_ERR_NOT_FOUND,
			RemoteError::UnknownAlgorithm => BKB_ERR_UNKNOWN_ALGORITHM,
			RemoteError::Cancelled => BKB_ERR_CANCELLED,
			RemoteError::Server(_) | RemoteError::UnexpectedStatus(_) => {
				BKB_ERR_REMOTE_BAD_STATUS_CODE
			}
		},
		StoreError::DownloadTaskFailed(_) => BKB_ERR_DOWNLOAD_TASK_FAILED,
	}
}

pub(crate) fn error_string(code: bkb_error_code) -> &'static CStr {
	match code {
		BKB_OK => c"success",
		BKB_ERR_NULL_ARG => c"null unexpectedly passed in as an argument",
		BKB_ERR_DB => c"database error",
		BKB_ERR_CORRUPT => c"corrupt database",
		BKB_ERR_MIGRATE => c"migration error",
		BKB_ERR_IO => c"I/O error",
		BKB_ERR_CONFIG => c"configuration error",
		BKB_ERR_JSON => c"JSON error",
		BKB_ERR_INVALID_COLLECTION_HEADER => c"invalid collection header",
		BKB_ERR_INVALID_BUNDLE => c"invalid bundle",
		BKB_ERR_TOO_LARGE => c"too large",
		BKB_ERR_ZIP => c"ZIP error",
		BKB_ERR_NESTED_BBZIP => c"nested bbzip",
		BKB_ERR_PARSE => c"invalid value provided - cannot parse",
		BKB_ERR_NOT_FOUND => c"not found",
		BKB_ERR_HASH_MISMATCH => c"hash mismatch",
		BKB_ERR_INVALID_PRECOMBINED_ASSETS => c"invalid precombined assets",
		BKB_ERR_INVALID_ASSET_PATH => c"invalid asset path",
		BKB_ERR_ID_MISMATCH => c"ID mismatch",
		BKB_ERR_NO_SERVERS => c"no remote servers",
		BKB_ERR_INVALID_URL => c"invalid URL",
		BKB_ERR_NETWORK => c"network error",
		BKB_ERR_UNKNOWN_ALGORITHM => c"unknown algorithm",
		BKB_ERR_PERMISSION_DENIED => c"permission denied",
		BKB_ERR_CANCELLED => c"cancelled",
		BKB_ERR_REMOTE_BAD_STATUS_CODE => c"remote server returned bad status code",
		BKB_ERR_DOWNLOAD_TASK_FAILED => c"download task failed",
		BKB_ERR_PANIC => c"panic in rust code. not good!",
		BKB_ERR_INVALID_STRING => c"invalid string passed in",
		BKB_ERR_INCOMPATIBLE_SQLITE => c"incompatible SQLite library",
		_ => c"unknown error",
	}
}

#[cfg(test)]
mod tests {
	use std::io;

	use super::*;

	fn io_error() -> io::Error {
		io::Error::other("test")
	}

	#[test]
	fn nested_io_errors_share_one_code() {
		let errors = [
			StoreError::Io(io_error()),
			StoreError::Config(ConfigError::Io(io_error())),
			StoreError::Bundle(BackbeatFileError::Io(io_error())),
			StoreError::BbZip(BbZipReadError::Io(io_error())),
			StoreError::BbZip(BbZipReadError::InvalidBackbeatEntry {
				entry_name: "test.bb".into(),
				source: BackbeatFileError::Io(io_error()),
			}),
			StoreError::BbZipWrite(BbZipWriteError::Io(io_error())),
			StoreError::PrecombinedAssets(PrecombinedAssetsError::Io(io_error())),
		];
		for error in errors {
			assert_eq!(store_error_code(&error), BKB_ERR_IO);
		}
	}

	#[test]
	fn bundle_errors_ignore_archive_nesting() {
		let direct = StoreError::Bundle(BackbeatFileError::ChartNotBase64);
		let nested = StoreError::BbZip(BbZipReadError::InvalidBackbeatEntry {
			entry_name: "test.bb".into(),
			source: BackbeatFileError::ChartNotGzip(io_error()),
		});
		assert_eq!(store_error_code(&direct), BKB_ERR_INVALID_BUNDLE);
		assert_eq!(store_error_code(&nested), BKB_ERR_INVALID_BUNDLE);
	}

	#[test]
	fn precombined_structure_errors_share_one_code() {
		let non_file = StoreError::PrecombinedAssets(PrecombinedAssetsError::NonFileEntry);
		let missing =
			StoreError::PrecombinedAssets(PrecombinedAssetsError::MissingPaths(Vec::new()));
		assert_eq!(
			store_error_code(&non_file),
			BKB_ERR_INVALID_PRECOMBINED_ASSETS
		);
		assert_eq!(
			store_error_code(&missing),
			BKB_ERR_INVALID_PRECOMBINED_ASSETS
		);
	}

	#[test]
	fn remote_errors_use_semantic_codes() {
		let local_missing = StoreError::NotFound("test".into());
		let remote_missing = StoreError::Remote(RemoteError::NotFound);
		let server = StoreError::Remote(RemoteError::Server(500));
		let status = StoreError::Remote(RemoteError::UnexpectedStatus(418));
		assert_eq!(store_error_code(&local_missing), BKB_ERR_NOT_FOUND);
		assert_eq!(store_error_code(&remote_missing), BKB_ERR_NOT_FOUND);
		assert_eq!(store_error_code(&server), BKB_ERR_REMOTE_BAD_STATUS_CODE);
		assert_eq!(store_error_code(&status), BKB_ERR_REMOTE_BAD_STATUS_CODE);
	}

	#[test]
	fn collection_http_status_uses_bad_status_code() {
		let error = StoreError::CollectionClient(CollectionClientError::HttpStatus {
			url: "https://example.com/header.json".into(),
			status: 404,
		});
		assert_eq!(store_error_code(&error), BKB_ERR_REMOTE_BAD_STATUS_CODE);
	}
}
