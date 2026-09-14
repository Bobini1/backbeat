use super::DEFAULT_BPM;

/// A parsed TJA chart file.
#[derive(Debug, Clone)]
pub struct Chart {
	/// Song-level metadata.
	pub metadata: Metadata,
	/// One entry per COURSE block found in the file.
	pub courses: Vec<Course>,
}

/// Song-level metadata from TJA global headers.
#[derive(Debug, Clone)]
pub struct Metadata {
	/// `TITLE:`
	pub title: Option<String>,
	/// `SUBTITLE:` (with leading `--` / `++` stripped).
	pub subtitle: Option<String>,
	/// `BPM:` — initial tempo. Default `120.0`.
	pub bpm: f64,
	/// `WAVE:` — BGM audio filename (first occurrence).
	pub wave: Option<String>,
	/// `OFFSET:` converted to milliseconds as `(seconds * 1000) as i32`.
	pub offset_ms: i32,
	/// `MOVIEOFFSET:` in milliseconds.
	pub movie_offset_ms: Option<i32>,
	/// `GENRE:`
	pub genre: Option<String>,
	/// `DEMOSTART:` in seconds.
	pub demo_start: Option<f64>,
	/// `BGMOVIE:` — background video filename.
	pub bg_movie: Option<String>,
	/// `BGIMAGE:` — background image filename.
	pub bg_image: Option<String>,
	/// `HIDDENBRANCH:` — any non-empty value sets this to `true`.
	pub hidden_branch: bool,
}

impl Default for Metadata {
	fn default() -> Self {
		Self {
			title: None,
			subtitle: None,
			bpm: DEFAULT_BPM,
			wave: None,
			offset_ms: 0,
			movie_offset_ms: None,
			genre: None,
			demo_start: None,
			bg_movie: None,
			bg_image: None,
			hidden_branch: false,
		}
	}
}

/// A single difficulty chart within a TJA file.
#[derive(Debug, Clone)]
pub struct Course {
	/// The difficulty level (Easy … Dan).
	pub difficulty: Difficulty,
	/// `LEVEL:` — numeric difficulty rating. Default `0`.
	pub level: u32,
	/// `BALLOON:` / `BALLOONNOR:` — hit counts for Normal-branch balloons.
	pub balloon_normal: Vec<u32>,
	/// `BALLOONEXP:` — hit counts for Expert-branch balloons.
	pub balloon_expert: Vec<u32>,
	/// `BALLOONMAS:` — hit counts for Master-branch balloons.
	pub balloon_master: Vec<u32>,
	/// `SCOREINIT:` — `[init_low, init_high]`. Default `[300, 1000]`.
	pub score_init: [u16; 2],
	/// `SCOREDIFF:` — score per note. Default `120`.
	pub score_diff: u16,
	/// `true` if the course contains a `#BRANCHSTART` command.
	pub has_branches: bool,
	/// All note and event objects parsed from the first `#START … #END` block.
	pub notes: Vec<Note>,
}

/// The seven difficulty slots defined by TJAPlayer3's `Difficulty` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Difficulty {
	/// Beginner difficulty.
	Easy = 0,
	/// Normal difficulty.
	Normal = 1,
	/// Hard difficulty.
	Hard = 2,
	/// Oni (Demon) difficulty — the default when no `COURSE:` header appears.
	Oni = 3,
	/// Edit (user-created) difficulty.
	Edit = 4,
	/// Tower mode difficulty.
	Tower = 5,
	/// Dan (exam) course difficulty.
	Dan = 6,
}

impl Difficulty {
	/// Convert from a 0-based index (clamped to Oni on unknown values).
	pub fn from_index(i: usize) -> Self {
		match i {
			0 => Self::Easy,
			1 => Self::Normal,
			2 => Self::Hard,
			3 => Self::Oni,
			4 => Self::Edit,
			5 => Self::Tower,
			6 => Self::Dan,
			_ => Self::Oni,
		}
	}

	/// Short display name (English, matching TJAPlayer3's difficulty strings).
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Easy => "Easy",
			Self::Normal => "Normal",
			Self::Hard => "Hard",
			Self::Oni => "Oni",
			Self::Edit => "Edit",
			Self::Tower => "Tower",
			Self::Dan => "Dan",
		}
	}
}

/// A single event placed in the chart timeline.
#[derive(Debug, Clone)]
pub struct Note {
	/// Measure number (1-based, matching TJAPlayer3's `n現在の小節数`).
	pub measure: u32,
	/// Tick within the measure: `measure * 384 + 384 * char_index / measure_width`.
	pub tick: u32,
	/// What kind of event this is.
	pub kind: NoteKind,
}

/// The type of a chart event.
#[derive(Debug, Clone, PartialEq)]
pub enum NoteKind {
	/// `1` — Don (small red).
	Don,
	/// `2` — Ka (small blue).
	Ka,
	/// `3` — Don (large).
	DonLarge,
	/// `4` — Ka (large).
	KaLarge,
	/// `5` — roll start.
	Roll,
	/// `6` — roll (large) start.
	RollLarge,
	/// `7` or `9` — balloon (potato `9` maps to balloon per TJAPlayer3 2017.01.30).
	Balloon,
	/// `8` — roll / balloon end.
	RollEnd,
	/// `A` — hand-held (手つなぎ).
	HandHeld,
	/// `B` — note type 11 (未使用).
	Special,
	/// `F` — ad-lib note.
	AdLib,
	/// `#GOGOSTART`
	GogoStart,
	/// `#GOGOEND`
	GogoEnd,
	/// `#BPMCHANGE <bpm>`
	BpmChange(f64),
	/// `#MEASURE <num>/<den>`
	Measure {
		/// Time signature numerator.
		numerator: u32,
		/// Time signature denominator.
		denominator: u32,
	},
	/// `#DELAY <seconds>`
	Delay(f64),
	/// `#SECTION`
	Section,
	/// `#BRANCHSTART <kind>,<a>,<b>`
	BranchStart {
		/// Branching condition type.
		kind: BranchKind,
		/// Lower threshold (Normal → Expert boundary).
		threshold_a: f64,
		/// Upper threshold (Expert → Master boundary).
		threshold_b: f64,
	},
	/// `#BRANCHEND`
	BranchEnd,
	/// `#BARLINEOFF`
	BarlineOff,
	/// `#BARLINEON`
	BarlineOn,
}

/// Branch condition type for `#BRANCHSTART`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchKind {
	/// `p` — percentage of notes hit.
	Percent,
	/// `r` — drumroll count.
	Roll,
	/// `s` — score.
	Score,
	/// `d` — drumroll (alternate).
	Drumroll,
}
