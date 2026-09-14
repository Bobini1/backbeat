/// Every metadata declaration in a BMS file, preserving source order and duplicates.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RawBmsHeaders {
	entries: Vec<(String, String)>,
}

impl RawBmsHeaders {
	/// Add a raw header name and value.
	pub(super) fn push(&mut self, key: String, value: String) {
		self.entries.push((key, value));
	}

	/// Get the first value for `tag`, treating tag names as case-insensitive.
	///
	/// An empty first value is treated as absent.
	pub fn get_first_tag(&self, tag: &str) -> Option<&str> {
		let tag = tag.to_ascii_uppercase();
		let (_, value) = self.entries.iter().find(|(key, _)| *key == tag)?;

		if value.is_empty() {
			return None;
		}

		Some(value.as_str())
	}

	/// Iterate over the header names and values.
	pub fn iter(&self) -> RawBmsHeadersIter<'_> {
		RawBmsHeadersIter {
			inner: self.entries.iter(),
		}
	}

	/// Return the number of metadata declarations.
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Return whether there are no metadata declarations.
	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

impl IntoIterator for RawBmsHeaders {
	type Item = (String, String);
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.entries.into_iter()
	}
}

impl<'a> IntoIterator for &'a RawBmsHeaders {
	type Item = (&'a str, &'a str);
	type IntoIter = RawBmsHeadersIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

/// An iterator over borrowed BMS metadata declarations.
pub struct RawBmsHeadersIter<'a> {
	inner: std::slice::Iter<'a, (String, String)>,
}

impl<'a> Iterator for RawBmsHeadersIter<'a> {
	type Item = (&'a str, &'a str);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner
			.next()
			.map(|(key, value)| (key.as_str(), value.as_str()))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}
}

impl ExactSizeIterator for RawBmsHeadersIter<'_> {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn get_first_tag() {
		let headers = RawBmsHeaders {
			entries: vec![
				("TITLE".to_owned(), "First".to_owned()),
				("TITLE".to_owned(), "Second".to_owned()),
			],
		};

		assert_eq!(headers.get_first_tag("title"), Some("First"));
		assert_eq!(headers.get_first_tag("ARTIST"), None);
	}

	#[test]
	fn get_first_tag_treats_empty_as_absent() {
		let headers = RawBmsHeaders {
			entries: vec![
				("TITLE".to_owned(), String::new()),
				("TITLE".to_owned(), "Second".to_owned()),
			],
		};

		assert_eq!(headers.get_first_tag("TITLE"), None);
	}
}
