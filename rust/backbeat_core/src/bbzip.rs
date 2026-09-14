use std::io::{self, Read, Seek, Write};
use std::path::Path;
use std::sync::LazyLock;

use zip::read::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::util::safe_extension;
use crate::{BackbeatFile, BackbeatFileError};

/// Read `.bbzip` files.
///
/// # Usage:
///
/// ```
/// use backbeat_core::BbZipReader;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # if false {
/// let file = std::fs::File::open("foo.bbzip")?;
/// let mut bbzip = BbZipReader::new(file)?;
///
/// for idx in 0..bbzip.len() {
///     let Some(entry) = bbzip.get_entry(idx)? else {
///         continue;
///     };
///
///     // do something with <entry>
/// }
/// # }
/// # Ok(())
/// # }
/// ```
pub struct BbZipReader {
	archive: ZipArchive<Box<dyn ReadSeek>>,
}

// rust hack so we can do `Box<dyn Read + Seek>`.
// there is a blanket impl here so who care
trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}
impl BbZipReader {
	pub const EXTENSION: &str = "bbzip";

	/// Create a new `.bbzip` from an already open reader. Pass in a File or Cursor
	/// into this.
	pub fn new<R>(reader: R) -> Result<Self, BbZipReadError>
	where
		R: Read + Seek + 'static,
	{
		let reader: Box<dyn ReadSeek> = Box::new(reader);

		Ok(Self {
			archive: ZipArchive::new(reader)?,
		})
	}

	#[expect(clippy::len_without_is_empty)]
	pub fn len(&mut self) -> usize {
		self.archive.len()
	}

	pub fn get_entry(&mut self, index: usize) -> Result<Option<BbZipEntry<'_>>, BbZipReadError> {
		let mut entry = self.archive.by_index(index).map_err(BbZipReadError::from)?;
		if !entry.is_file() {
			return Ok(None);
		}
		let entry_name = entry.name().to_owned();

		let result = match BbZipFiletype::new(&entry_name) {
			BbZipFiletype::Bb => {
				let mut bytes = Vec::new();
				entry
					.read_to_end(&mut bytes)
					.map_err(BbZipReadError::from)?;
				let bb = BackbeatFile::from_json(&bytes).map_err(|source| {
					BbZipReadError::InvalidBackbeatEntry {
						entry_name: entry_name.clone(),
						source,
					}
				})?;

				BbZipEntry::Bb(bb)
			}
			BbZipFiletype::Bbzip => return Err(BbZipReadError::NestedZip),
			BbZipFiletype::Asset => BbZipEntry::Asset(Box::new(entry)),
		};

		Ok(Some(result))
	}
}

/// An entry from the streaming reader in [`BbZipReader`].
pub enum BbZipEntry<'a> {
	Bb(BackbeatFile),
	Asset(Box<dyn Read + 'a>),
}

#[derive(Debug, Clone, Copy)]
enum BbZipFiletype {
	Bb,
	Bbzip,
	Asset,
}

impl BbZipFiletype {
	fn new(filename: &str) -> Self {
		let Some(sext) = safe_extension(Path::new(filename)) else {
			return Self::Asset;
		};

		match sext {
			BackbeatFile::EXTENSION => Self::Bb,
			BbZipReader::EXTENSION => Self::Bbzip,
			_ => Self::Asset,
		}
	}
}

/// Create `.bbzip` files.
pub struct BbZipWriter<W: Write + Seek> {
	archive: ZipWriter<W>,
}

static COMMENT: LazyLock<String> = LazyLock::new(|| format!("bkb_core/{}", crate::VERSION));

impl<W: Write + Seek> BbZipWriter<W> {
	pub fn new(output: W) -> Self {
		let mut writer = ZipWriter::new(output);
		writer = writer.set_auto_large_file();

		writer
			.set_comment(&**COMMENT)
			.expect("comment is less than max comment bytes");

		Self { archive: writer }
	}

	/// Just add a file to the zip with this filename and these contents.
	///
	/// Don't add the same file under the same name multiple times.
	pub fn add_file(
		&mut self,
		filename: impl AsRef<Path>,
		mut contents: impl Read,
	) -> Result<(), BbZipWriteError> {
		let filename = filename.as_ref();

		if filename.ends_with(".bbzip") {
			return Err(BbZipWriteError::NestedZip);
		}

		self.archive
			.start_file_from_path(filename, SimpleFileOptions::default())?;
		io::copy(&mut contents, &mut self.archive)?;

		Ok(())
	}

	/// Finish the archive and return the output writer.
	pub fn finish(self) -> Result<W, BbZipWriteError> {
		Ok(self.archive.finish()?)
	}
}

/// Errors that can occur while reading or writing a `.bbzip` archive.
#[derive(Debug, thiserror::Error)]
pub enum BbZipReadError {
	#[error("ZIP error: {0}")]
	Zip(#[from] zip::result::ZipError),

	#[error("I/O error: {0}")]
	Io(#[from] io::Error),

	#[error("invalid .bb entry {entry_name:?}: {source}")]
	InvalidBackbeatEntry {
		entry_name: String,
		#[source]
		source: BackbeatFileError,
	},

	#[error(".bbzip files cannot contain other .bbzip files.")]
	NestedZip,
}

#[derive(Debug, thiserror::Error)]
pub enum BbZipWriteError {
	#[error("ZIP error: {0}")]
	Zip(#[from] zip::result::ZipError),

	#[error("I/O error: {0}")]
	Io(#[from] io::Error),

	#[error("you cannot put .bbzip files inside .bbzip files, you lunatic")]
	NestedZip,
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::io::{Cursor, Write};

	use super::*;
	use crate::{ChartData, ChartDesc, ChartFilename};

	fn bb(title: &str) -> BackbeatFile {
		BackbeatFile {
			filename: ChartFilename::from_path("chart.bms").unwrap(),
			assets: HashMap::new(),
			desc: ChartDesc::new(title).unwrap(),
			chart: ChartData::compress(format!("#TITLE {title}\n").as_bytes()).unwrap(),
		}
	}

	fn raw_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
		let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
		for (name, data) in entries {
			archive
				.start_file(name, SimpleFileOptions::default())
				.unwrap();
			archive.write_all(data).unwrap();
		}
		archive.finish().unwrap().into_inner()
	}

	#[test]
	fn reads_manifests_by_extension_and_streams_other_entries() {
		let first = bb("First");
		let second = bb("Second");
		let bytes = raw_archive(&[
			("assets/anything", b"asset one"),
			("wherever/first.bb", &first.to_json()),
			("metadata.json", b"{\"not\":\"a manifest\"}"),
			("second.bb", &second.to_json()),
		]);

		let mut manifests = Vec::new();
		let mut assets = Vec::new();
		let mut archive = BbZipReader::new(Cursor::new(bytes)).unwrap();
		for index in 0..archive.len() {
			match archive.get_entry(index).unwrap().unwrap() {
				BbZipEntry::Bb(manifest) => manifests.push(manifest),
				BbZipEntry::Asset(mut asset) => {
					let mut bytes = Vec::new();
					asset.read_to_end(&mut bytes).unwrap();
					assets.push(bytes);
				}
			}
		}

		assert_eq!(manifests, vec![first, second]);
		assert_eq!(
			assets,
			vec![b"asset one".to_vec(), b"{\"not\":\"a manifest\"}".to_vec()]
		);
	}

	#[test]
	fn rejects_invalid_bb_entries() {
		let bytes = raw_archive(&[("manifest.bb", b"not json")]);
		let mut archive = BbZipReader::new(Cursor::new(bytes)).unwrap();
		let error = match archive.get_entry(0) {
			Err(error) => error,
			Ok(_) => panic!("invalid manifest should fail"),
		};
		assert!(matches!(
			error,
			BbZipReadError::InvalidBackbeatEntry { entry_name, .. } if entry_name == "manifest.bb"
		));
	}
}
