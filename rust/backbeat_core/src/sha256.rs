//! Utilities for performing checksums.

use std::fmt::{Debug, Display};
use std::io;
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use serde::de::{Unexpected, Visitor};
use sha2::Digest;

use crate::File;

/// How many bytes are in a sha256 checksum?
pub const SHA256_BYTES: usize = 32;

/// Utility struct for handling Sha256 checksums. This struct
/// enforces the validity of the checksums, and provides
/// Display/Debug/Serialize implementations that are convenient.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Sha256([u8; SHA256_BYTES]);

impl Debug for Sha256 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Sha256").field(&hex::encode(self.0)).finish()
	}
}

#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("expected a lowercase sha256 checksum")]
pub struct ParseSha256Error;

impl From<[u8; SHA256_BYTES]> for Sha256 {
	fn from(value: [u8; SHA256_BYTES]) -> Self {
		Self(value)
	}
}

impl FromStr for Sha256 {
	type Err = ParseSha256Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::from_lowercase_hex(s).ok_or(ParseSha256Error)
	}
}

impl serde::Serialize for Sha256 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Sha256 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Sha256Visitor;

		impl Visitor<'_> for Sha256Visitor {
			type Value = Sha256;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a sha256 string, all lowercase")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Sha256::from_lowercase_hex(v).ok_or_else(|| {
					E::invalid_value(
						Unexpected::Str(v),
						&"wasn't a sha256 string; should be 32 chars long and contain only 0-9, \
						  a-f",
					)
				})
			}
		}

		deserializer.deserialize_str(Sha256Visitor)
	}
}

impl Display for Sha256 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&hex::encode(self.0))
	}
}

impl Sha256 {
	/// Get the empty sha256 hash. This can be used in rare circumstances where we need
	/// a hash but have no data to give.
	pub const fn null() -> Self {
		Self([0; SHA256_BYTES])
	}

	/// Turn this stream of bytes into sha256.
	pub fn checksum_data(mut content: impl io::Read) -> io::Result<Self> {
		let mut hasher = sha2::Sha256::new();

		io::copy(&mut content, &mut hasher)?;

		let bytes: [u8; SHA256_BYTES] = hasher
			.finalize()
			.as_slice()
			.try_into()
			.expect("got invalid sha256 result");

		Ok(Self(bytes))
	}

	/// Create a sha256 object from a lowercase hex string (0-9a-f).
	///
	/// This is the canonical representation of a checksum in Zenith.
	///
	/// If the string provided is invalid, None is returned.
	fn from_lowercase_hex(str: &str) -> Option<Self> {
		if !SHA256_REGEX.is_match(str) {
			return None;
		}

		let bytes = hex::decode(str).ok()?;
		let bytes: [u8; SHA256_BYTES] = bytes.as_slice().try_into().ok()?;

		Some(Self(bytes))
	}

	/// Sha256 these bytes. This is infallible, as no copy operations
	/// take place. However, buffering bytes into memory is expensive.
	///
	/// You likely want [`Sha256::checksum_data`], but this function exists
	/// to avoid annoying [`std::io::Error`]s that cannot happen.
	pub fn checksum_bytes(bytes: &[u8]) -> Self {
		let mut hasher = sha2::Sha256::new();

		hasher.update(bytes);

		let bytes = hasher
			.finalize()
			.as_slice()
			.try_into()
			.expect("got invalid sha256 result");

		Self(bytes)
	}

	/// Open the file at this path and turn it into a sha256 object.
	pub fn open_and_checksum(path: impl AsRef<Path>) -> io::Result<Self> {
		let path = path.as_ref().to_owned();
		let file = File::open(path)?;

		Self::checksum_data(file)
	}

	/// Get the underlying bytes for this checksum.
	pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
		&self.0
	}
}

/// A regex for validating sha256 strings.
pub static SHA256_REGEX: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"^[a-f0-9]{64}$").expect("invalid sha256 regexp"));

/// Calculate sha256 in a rolling manner.
///
/// Write more bytes with [`Sha256Digest::update`], then finalize the stream
/// with [`Sha256Digest::finalize`].
pub struct Sha256Digest {
	digest: sha2::Sha256,
}

impl Sha256Digest {
	pub fn new() -> Self {
		Self {
			digest: sha2::Sha256::new(),
		}
	}

	/// Write some new data into the digest.
	pub fn update(&mut self, bytes: &[u8]) {
		self.digest.update(bytes);
	}

	/// Consume the digest, and get back a [`Sha256`] checksum.
	pub fn finalize(self) -> Sha256 {
		let bytes = self
			.digest
			.finalize()
			.as_slice()
			.try_into()
			.expect("got invalid sha256 result");

		Sha256(bytes)
	}
}

impl Default for Sha256Digest {
	fn default() -> Self {
		Self::new()
	}
}
