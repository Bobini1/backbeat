use super::LoadError;

// ── Notes ─────────────────────────────────────────────────────────────────────

/// State of one BT (button) lane in a chart row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtNote {
	/// No note (`0`).
	None,
	/// Chip note (`1`).
	Chip,
	/// Hold note start or continuation (`2`).
	Hold,
}

impl BtNote {
	pub(super) fn from_char(c: u8) -> Self {
		match c {
			b'1' => Self::Chip,
			b'2' => Self::Hold,
			_ => Self::None,
		}
	}
}

/// State of one FX lane in a chart row.
///
/// Note that `1`/long and `2`/chip are **reversed** relative to BT lanes (for
/// historical reasons). Legacy alphabetic characters (`S`, `V`, `G`, …) all
/// map to [`Hold`](FxNote::Hold).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxNote {
	/// No note (`0`).
	None,
	/// Chip FX note (`2`).
	Chip,
	/// Long FX note (`1` or any legacy letter).
	Hold,
}

impl FxNote {
	pub(super) fn from_char(c: u8) -> Self {
		match c {
			b'0' => Self::None,
			b'2' => Self::Chip,
			_ => Self::Hold, // '1' and all legacy letters
		}
	}
}

/// State of one laser lane in a chart row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaserCell {
	/// No laser (`-`).
	None,
	/// Continuation of the previous laser segment (`:`).
	Continuation,
	/// Laser knob at a specific position (0 = leftmost, 50 = rightmost).
	Position(u8),
}

/// 51-character laser position grid (left → right).
const LASER_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmno";

impl LaserCell {
	pub(super) fn from_char(c: u8) -> Self {
		match c {
			b'-' => Self::None,
			b':' => Self::Continuation,
			_ => LASER_CHARS
				.iter()
				.position(|&x| x == c)
				.map(|i| Self::Position(i as u8))
				.unwrap_or(Self::None),
		}
	}
}

// ── Lane spin ─────────────────────────────────────────────────────────────────

/// Type of lane spin or swing effect appended to a chart row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneSpinKind {
	/// `@(` — Normal spin, left (clockwise).
	NormalLeft,
	/// `@)` — Normal spin, right (counterclockwise).
	NormalRight,
	/// `@<` — Half spin, left.
	HalfLeft,
	/// `@>` — Half spin, right.
	HalfRight,
	/// `S<` — Swing effect, left.
	SwingLeft,
	/// `S>` — Swing effect, right.
	SwingRight,
}

/// A lane spin or swing effect attached to a chart row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneSpin {
	pub kind: LaneSpinKind,
	/// Duration in pulses (192 per measure).
	pub length: u32,
	/// Swing scale value (default 250). Only meaningful for swing kinds.
	pub scale: Option<i32>,
	/// Number of swing repetitions (default 3). Only meaningful for swing kinds.
	pub repetitions: Option<i32>,
	/// Swing decay order 0–2 (default 2). Only meaningful for swing kinds.
	pub decay_order: Option<u8>,
}

impl LaneSpin {
	pub(super) fn parse(s: &str) -> Option<Self> {
		if s.len() < 3 {
			return None;
		}
		let kind = match s.get(..2)? {
			"@(" => LaneSpinKind::NormalLeft,
			"@)" => LaneSpinKind::NormalRight,
			"@<" => LaneSpinKind::HalfLeft,
			"@>" => LaneSpinKind::HalfRight,
			"S<" => LaneSpinKind::SwingLeft,
			"S>" => LaneSpinKind::SwingRight,
			_ => return None,
		};

		let rest = s.get(2..)?;
		let mut parts = rest.splitn(4, ';');
		let length: u32 = parts.next()?.parse().ok()?;
		let scale = parts.next().and_then(|p| p.parse().ok());
		let repetitions = parts.next().and_then(|p| p.parse().ok());
		let decay_order = parts.next().and_then(|p| p.parse().ok());

		Some(Self {
			kind,
			length,
			scale,
			repetitions,
			decay_order,
		})
	}
}

// ── Chart row ─────────────────────────────────────────────────────────────────

/// A single row of chart data (one pulse line in a measure).
///
/// Format: `<bt×4>|<fx×2>|<laser×2>[spin]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
	pub bt: [BtNote; 4],
	pub fx: [FxNote; 2],
	pub laser: [LaserCell; 2],
	pub spin: Option<LaneSpin>,
}

impl Row {
	/// Returns `true` if `line` looks like a chart row (exactly two `|` pipes).
	pub(super) fn is_chart_line(line: &str) -> bool {
		line.bytes().filter(|&b| b == b'|').count() == 2
	}

	pub(super) fn parse(line: &str, line_num: usize) -> Result<Self, LoadError> {
		let err = || LoadError::InvalidChartLine { line: line_num };

		let mut parts = line.splitn(3, '|');
		let bt_str = parts.next().ok_or_else(err)?;
		let fx_str = parts.next().ok_or_else(err)?;
		let laser_spin_str = parts.next().ok_or_else(err)?;

		if bt_str.len() != 4 || fx_str.len() < 2 || laser_spin_str.len() < 2 {
			return Err(err());
		}

		let bt_b = bt_str.as_bytes();
		let bt = [
			BtNote::from_char(bt_b[0]),
			BtNote::from_char(bt_b[1]),
			BtNote::from_char(bt_b[2]),
			BtNote::from_char(bt_b[3]),
		];

		let fx_b = fx_str.as_bytes();
		let fx = [FxNote::from_char(fx_b[0]), FxNote::from_char(fx_b[1])];

		let laser_b = laser_spin_str.as_bytes();
		let laser = [
			LaserCell::from_char(laser_b[0]),
			LaserCell::from_char(laser_b[1]),
		];

		let spin = if laser_spin_str.len() > 2 {
			laser_spin_str.get(2..).and_then(LaneSpin::parse)
		} else {
			None
		};

		Ok(Self {
			bt,
			fx,
			laser,
			spin,
		})
	}
}
