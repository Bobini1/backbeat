use super::parse_beat;
use super::row::Row;

// ── Tilt ──────────────────────────────────────────────────────────────────────

/// Lane tilt mode.
#[derive(Debug, Clone, PartialEq)]
pub enum TiltValue {
	Normal,
	Bigger,
	Biggest,
	KeepNormal,
	KeepBigger,
	KeepBiggest,
	Zero,
	/// Manual tilt amount (float, applied as a linear graph).
	Manual(f64),
}

impl TiltValue {
	pub(super) fn parse(value: &str) -> Self {
		match value {
			"normal" => Self::Normal,
			"bigger" | "big" => Self::Bigger,
			"biggest" => Self::Biggest,
			"keep_normal" => Self::KeepNormal,
			"keep_bigger" | "keep" => Self::KeepBigger,
			"keep_biggest" => Self::KeepBiggest,
			"zero" => Self::Zero,
			other => other
				.parse::<f64>()
				.map(Self::Manual)
				.unwrap_or(Self::Normal),
		}
	}
}

// ── Body option ───────────────────────────────────────────────────────────────

/// An option line that appears inside the chart body (after the first `--`).
#[derive(Debug, Clone, PartialEq)]
pub enum BodyOption {
	/// `t=` BPM change.
	Bpm(f64),
	/// `beat=` time signature change (numerator, denominator).
	Beat(u32, u32),
	/// `fx-l=` audio effect for the left long FX note.
	FxLeft(String),
	/// `fx-r=` audio effect for the right long FX note.
	FxRight(String),
	/// `filtertype=` laser audio effect.
	FilterType(String),
	/// `laserrange_l=2x` — enable wide-range left laser.
	LaserRangeLeft,
	/// `laserrange_r=2x` — enable wide-range right laser.
	LaserRangeRight,
	/// `zoom_top=` camera control.
	ZoomTop(i32),
	/// `zoom_bottom=` camera control.
	ZoomBottom(i32),
	/// `zoom_side=` camera control (horizontal position shift).
	ZoomSide(i32),
	/// `center_split=` highway centre-margin control.
	CenterSplit(f64),
	/// `tilt=` lane tilt control.
	Tilt(TiltValue),
	/// `scroll_speed=` scroll speed multiplier.
	ScrollSpeed(f64),
	/// `rotation_deg=` rotation angle in degrees.
	RotationDeg(f64),
	/// `stop=` pause duration in pulses.
	Stop(u32),
	/// `chokkakuvol=` laser slam volume (0–100).
	ChokkakuVol(u8),
	/// `chokkakuse=` laser slam sound override.
	ChokkakuSe(String),
	/// `pfiltergain=` peaking filter gain (0–100).
	PFilterGain(u8),
	/// Any key/value pair not recognised by this parser.
	Unknown { key: String, value: String },
}

impl BodyOption {
	pub(super) fn parse(key: &str, value: &str) -> Self {
		// Reduce boilerplate for "parse number or fall back to Unknown".
		macro_rules! parse_num {
			($ty:ty, $variant:expr) => {
				value
					.parse::<$ty>()
					.map($variant)
					.unwrap_or_else(|_| Self::Unknown {
						key: key.to_owned(),
						value: value.to_owned(),
					})
			};
		}

		match key {
			"t" => parse_num!(f64, Self::Bpm),
			"beat" => parse_beat(value)
				.map(|(n, d)| Self::Beat(n, d))
				.unwrap_or_else(|| Self::Unknown {
					key: key.to_owned(),
					value: value.to_owned(),
				}),
			"fx-l" => Self::FxLeft(value.to_owned()),
			"fx-r" => Self::FxRight(value.to_owned()),
			"filtertype" => Self::FilterType(value.to_owned()),
			"laserrange_l" => Self::LaserRangeLeft,
			"laserrange_r" => Self::LaserRangeRight,
			"zoom_top" => parse_num!(i32, Self::ZoomTop),
			"zoom_bottom" => parse_num!(i32, Self::ZoomBottom),
			"zoom_side" => parse_num!(i32, Self::ZoomSide),
			"center_split" => parse_num!(f64, Self::CenterSplit),
			"tilt" => Self::Tilt(TiltValue::parse(value)),
			"scroll_speed" => parse_num!(f64, Self::ScrollSpeed),
			"rotation_deg" => parse_num!(f64, Self::RotationDeg),
			"stop" => parse_num!(u32, Self::Stop),
			"chokkakuvol" => parse_num!(u8, Self::ChokkakuVol),
			"chokkakuse" => Self::ChokkakuSe(value.to_owned()),
			"pfiltergain" => parse_num!(u8, Self::PFilterGain),
			_ => Self::Unknown {
				key: key.to_owned(),
				value: value.to_owned(),
			},
		}
	}
}

// ── Measure ───────────────────────────────────────────────────────────────────

/// An event occurring within a measure: either a body option or a chart row.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasureEvent {
	Option(BodyOption),
	Row(Row),
}

/// All chart data between two consecutive `--` bar lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
	pub events: Vec<MeasureEvent>,
}
