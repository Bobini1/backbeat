// ── Definitions ───────────────────────────────────────────────────────────────

/// Footer definition type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
	/// `#define_fx`
	Fx,
	/// `#define_filter`
	Filter,
}

/// A `#define_fx` or `#define_filter` footer entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
	pub kind: DefinitionKind,
	pub name: String,
	pub value: String,
}

impl Definition {
	pub(super) fn try_parse(line: &str) -> Option<Self> {
		let rest = line.strip_prefix("#define_")?;
		let (kind, rest) = if let Some(r) = rest.strip_prefix("fx ") {
			(DefinitionKind::Fx, r)
		} else {
			let r = rest.strip_prefix("filter ")?;
			(DefinitionKind::Filter, r)
		};

		// Format after the kind prefix: `<name> <value>` (first space is the sep).
		let (name, value) = rest.split_once(' ')?;
		Some(Self {
			kind,
			name: name.to_owned(),
			value: value.to_owned(),
		})
	}
}
