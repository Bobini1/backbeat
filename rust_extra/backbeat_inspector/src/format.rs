#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecognisedFormat {
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

impl RecognisedFormat {
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

	pub(crate) fn from_filename(filename: &str) -> Option<Self> {
		let extension = filename.rsplit_once('.')?.1;
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
}
