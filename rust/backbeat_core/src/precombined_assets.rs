//! The streaming `.tar.gz` format used by precombined-assets endpoints.

use std::collections::HashSet;
use std::io::{self, Read};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{AssetId, AssetPath, AssetPathError, Assets, Sha256Digest};

/// Read, validate and generally unpack a precombined-assets file.
///
/// This takes in a tar.gz reader, and you will get a callback for each asset that is decoded
/// from the precombined file.
pub fn read_archive<R>(
	expected_assets: &Assets,
	reader: R,
	mut on_asset: impl FnMut(&AssetPath, AssetId, &mut dyn Read) -> io::Result<()>,
) -> Result<(), PrecombinedAssetsError>
where
	R: Read,
{
	let mut archive = Archive::new(GzDecoder::new(reader));
	let mut seen = HashSet::new();

	for entry in archive.entries()? {
		let mut entry = entry?;
		if !entry.header().entry_type().is_file() {
			return Err(PrecombinedAssetsError::NonFileEntry);
		}

		let path_bytes = entry.path_bytes();
		let path_string =
			std::str::from_utf8(&path_bytes).map_err(|_| PrecombinedAssetsError::NonUtf8Path)?;
		let path = AssetPath::new(path_string).map_err(|source| {
			PrecombinedAssetsError::InvalidAssetPath {
				path: path_string.to_owned(),
				source,
			}
		})?;
		let expected = *expected_assets
			.get(&path)
			.ok_or_else(|| PrecombinedAssetsError::UnexpectedPath(path.clone()))?;
		if !seen.insert(path.clone()) {
			return Err(PrecombinedAssetsError::DuplicatePath(path));
		}

		let len = entry.size();
		let mut reader = HashingReader::new(&mut entry);
		on_asset(&path, expected, &mut reader)?;

		if reader.bytes_read() != len {
			return Err(PrecombinedAssetsError::AssetNotFullyRead(path));
		}

		let actual = AssetId(reader.finish());
		if actual != expected {
			return Err(PrecombinedAssetsError::AssetHashMismatch {
				path,
				expected,
				actual,
			});
		}
	}

	let mut missing: Vec<AssetPath> = expected_assets
		.keys()
		.filter(|path| !seen.contains(*path))
		.cloned()
		.collect();
	missing.sort_unstable();
	if missing.is_empty() {
		Ok(())
	} else {
		Err(PrecombinedAssetsError::MissingPaths(missing))
	}
}

#[derive(Debug, thiserror::Error)]
pub enum PrecombinedAssetsError {
	#[error("failed to read or write the precombined-assets archive: {0}")]
	Io(#[from] io::Error),

	#[error("precombined-assets archive contains a non-file entry")]
	NonFileEntry,

	#[error("precombined-assets archive entry name is not valid UTF-8")]
	NonUtf8Path,

	#[error("precombined-assets archive has invalid asset path {path:?}: {source}")]
	InvalidAssetPath {
		path: String,
		#[source]
		source: AssetPathError,
	},

	#[error("precombined-assets archive contains unexpected path {0}")]
	UnexpectedPath(AssetPath),

	#[error("precombined-assets archive contains path {0} more than once")]
	DuplicatePath(AssetPath),

	#[error("precombined-assets archive is missing paths: {0:?}")]
	MissingPaths(Vec<AssetPath>),

	#[error("precombined-assets archive entry {0} was not read completely")]
	AssetNotFullyRead(AssetPath),

	#[error("precombined-assets asset {path} hash mismatch: expected {expected}, got {actual}")]
	AssetHashMismatch {
		path: AssetPath,
		expected: AssetId,
		actual: AssetId,
	},
}

struct HashingReader<R> {
	inner: R,
	digest: Sha256Digest,
	bytes_read: u64,
}

impl<R> HashingReader<R> {
	fn new(inner: R) -> Self {
		Self {
			inner,
			digest: Sha256Digest::new(),
			bytes_read: 0,
		}
	}

	const fn bytes_read(&self) -> u64 {
		self.bytes_read
	}

	fn finish(self) -> crate::Sha256 {
		self.digest.finalize()
	}
}

impl<R: Read> Read for HashingReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		let read = self.inner.read(buf)?;
		self.digest.update(&buf[..read]);
		self.bytes_read += read as u64;
		Ok(read)
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::io::Cursor;
	use std::path::PathBuf;

	use super::*;
	use crate::Sha256;
	use flate2::Compression;
	use flate2::write::GzEncoder;
	use tar::{Builder, EntryType, Header};

	fn asset(bytes: &[u8]) -> AssetId {
		AssetId(Sha256::checksum_bytes(bytes))
	}

	fn path(path: &str) -> AssetPath {
		path.parse().unwrap()
	}

	#[test]
	fn archives_preserve_asset_paths_and_duplicate_payloads() {
		let music = b"music".to_vec();
		let music_id = asset(&music);
		let assets = HashMap::from([
			(path("music.ogg"), music_id),
			(path("preview.ogg"), music_id),
		]);

		let archive = raw_archive(&[("music.ogg", music.clone()), ("preview.ogg", music.clone())]);

		let mut tar = Archive::new(GzDecoder::new(Cursor::new(archive.clone())));
		let mut names = tar
			.entries()
			.unwrap()
			.map(|entry| entry.unwrap().path().unwrap().into_owned())
			.collect::<Vec<_>>();
		names.sort();
		assert_eq!(
			names,
			vec![PathBuf::from("music.ogg"), PathBuf::from("preview.ogg")]
		);

		let mut staged = Vec::new();
		read_archive(&assets, Cursor::new(archive), |path, _, reader| {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes)?;
			staged.push((path.clone(), bytes));
			Ok(())
		})
		.unwrap();
		assert_eq!(staged.len(), 2);
		assert!(staged.into_iter().all(|(_, data)| data == music));
	}

	#[test]
	fn archives_require_every_asset_path() {
		let music = b"music".to_vec();
		let music_id = asset(&music);
		let assets = HashMap::from([
			(path("music.ogg"), music_id),
			(path("preview.ogg"), music_id),
		]);

		let archive = raw_archive(&[("music.ogg", music)]);
		let err = read_archive(&assets, Cursor::new(archive), |_, _, reader| {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes)?;
			Ok(())
		})
		.unwrap_err();

		assert!(matches!(err, PrecombinedAssetsError::MissingPaths(_)));
	}

	fn raw_archive(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
		let encoder = GzEncoder::new(Vec::new(), Compression::default());
		let mut archive = Builder::new(encoder);
		for (path, data) in entries {
			let mut header = Header::new_gnu();
			header.set_entry_type(EntryType::Regular);
			header.set_size(data.len() as u64);
			header.set_cksum();
			archive
				.append_data(&mut header, path, Cursor::new(data))
				.unwrap();
		}
		archive.into_inner().unwrap().finish().unwrap()
	}
}
