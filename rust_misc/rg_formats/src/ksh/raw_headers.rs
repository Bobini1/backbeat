/// KSH header declarations in source order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RawHeaders {
	entries: Vec<(String, String)>,
}

impl RawHeaders {
	pub(super) fn from_text(text: &str) -> Self {
		let mut headers = Self::default();

		for line in text.lines().map(|line| line.trim_end_matches('\r')) {
			if line == "--" {
				break;
			}
			if line.is_empty() || line.starts_with("//") {
				continue;
			}

			let Some((key, value)) = line.split_once('=') else {
				continue;
			};
			headers.entries.push((key.to_owned(), value.to_owned()));
		}

		headers
	}

	/// Return the first value for `key`.
	pub fn get_first(&self, key: &str) -> Option<&str> {
		self.entries
			.iter()
			.find(|(entry_key, _)| entry_key == key)
			.map(|(_, value)| value.as_str())
	}

	/// Iterate over raw header keys and values in source order.
	pub fn iter(&self) -> RawHeadersIter<'_> {
		RawHeadersIter {
			inner: self.entries.iter(),
		}
	}

	/// Return the number of header declarations.
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Return whether there are no header declarations.
	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

impl<'a> IntoIterator for &'a RawHeaders {
	type Item = (&'a str, &'a str);
	type IntoIter = RawHeadersIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

/// Iterator over raw KSH header declarations.
pub struct RawHeadersIter<'a> {
	inner: std::slice::Iter<'a, (String, String)>,
}

impl<'a> Iterator for RawHeadersIter<'a> {
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

impl ExactSizeIterator for RawHeadersIter<'_> {}
