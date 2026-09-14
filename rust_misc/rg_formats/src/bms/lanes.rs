//! Handles [`Mode`] specific mappings for bms gamemodes.
//!
//! Different gamemodes choose to map different channels to different playable lanes
//! in game.

use super::{Channel, Mode, channels};

macro_rules! mappings {
	($ty:ty, $fn_name:ident, { $($key:ident -> $value:ident),* $(,)? }) => {
		impl $ty {
			/// Create a new lane for this gamemode.
			pub fn $fn_name(channel: Channel) -> Option<(Self, LaneKind)> {
				$(
					if channel == channels::$key {
						return Some((Self::$value, LaneKind::Note))
					}

					{
						let mut hold_channel = channels::$key;
						hold_channel.0 += 4;

						if channel == hold_channel {
							return Some((Self::$value, LaneKind::LongNote))
						}

						let mut invis_channel = channels::$key;
						invis_channel.0 += 2;

						if channel == invis_channel {
							return Some((Self::$value, LaneKind::KeysoundOnly))
						}

						let mut mine_channel = channels::$key;
						mine_channel.0 += 12;

						if channel == mine_channel {
							return Some((Self::$value, LaneKind::Mine))
						}
					}
				)*

				None
			}
		}
	};
}

/// Lanes available for the [`Mode::Beat5`] gamemode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Beat5Lane {
	B1,
	B2,
	B3,
	B4,
	B5,
	Scratch,
}

mappings!(Beat5Lane, new, {
	BEAT_K1 -> B1,
	BEAT_K2 -> B2,
	BEAT_K3 -> B3,
	BEAT_K4 -> B4,
	BEAT_K5 -> B5,
	BEAT_SCR -> Scratch,
});

/// Lanes available for the [`Mode::Beat7`] gamemode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Beat7Lane {
	B1,
	B2,
	B3,
	B4,
	B5,
	B6,
	B7,
	Scratch,
}

mappings!(Beat7Lane, new, {
	BEAT_K1 -> B1,
	BEAT_K2 -> B2,
	BEAT_K3 -> B3,
	BEAT_K4 -> B4,
	BEAT_K5 -> B5,
	BEAT_K6 -> B6,
	BEAT_K7 -> B7,
	BEAT_SCR -> Scratch
});

/// Lanes available for the [`Mode::Beat10`] gamemode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Beat10Lane {
	ScratchLeft,
	B1,
	B2,
	B3,
	B4,
	B5,
	B6,
	B7,
	B8,
	B9,
	B10,
	ScratchRight,
}

mappings!(Beat10Lane, new,  {
	BEAT_K1 -> B1,
	BEAT_K2 -> B2,
	BEAT_K3 -> B3,
	BEAT_K4 -> B4,
	BEAT_K5 -> B5,
	BEAT_DP_K1 -> B6,
	BEAT_DP_K2 -> B7,
	BEAT_DP_K3 -> B8,
	BEAT_DP_K4 -> B9,
	BEAT_DP_K5 -> B10,
	BEAT_SCR -> ScratchLeft,
	BEAT_DP_SCR -> ScratchRight
});

/// Lanes available for the [`Mode::Beat14`] gamemode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Beat14Lane {
	ScratchLeft,
	B1,
	B2,
	B3,
	B4,
	B5,
	B6,
	B7,
	B8,
	B9,
	B10,
	B11,
	B12,
	B13,
	B14,
	ScratchRight,
}

mappings!(Beat14Lane, new, {
	BEAT_K1 -> B1,
	BEAT_K2 -> B2,
	BEAT_K3 -> B3,
	BEAT_K4 -> B4,
	BEAT_K5 -> B5,
	BEAT_K6 -> B6,
	BEAT_K7 -> B7,
	BEAT_DP_K1 -> B8,
	BEAT_DP_K2 -> B9,
	BEAT_DP_K3 -> B10,
	BEAT_DP_K4 -> B11,
	BEAT_DP_K5 -> B12,
	BEAT_DP_K6 -> B13,
	BEAT_DP_K7 -> B14,
	BEAT_SCR -> ScratchLeft,
	BEAT_DP_SCR -> ScratchRight
});

/// Lanes available for the [`Mode::Pop5SP`] or [`Mode::Pop5DP`] modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Pop5Lane {
	B1,
	B2,
	B3,
	B4,
	B5,
}

mappings!(Pop5Lane, new_sp, {
	POP_3 -> B1,
	POP_4 -> B2,
	POP_5 -> B3,
	POP_6_SP -> B4,
	POP_7_SP -> B5,
});

mappings!(Pop5Lane, new_dp, {
	POP_3 -> B1,
	POP_4 -> B2,
	POP_5 -> B3,
	POP_6_DP -> B4,
	POP_7_DP -> B5,
});

/// Lanes available for the [`Mode::Pop9SP`] or [`Mode::Pop9DP`] gamemodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Pop9Lane {
	B1,
	B2,
	B3,
	B4,
	B5,
	B6,
	B7,
	B8,
	B9,
}

mappings!(Pop9Lane, new_sp, {
	POP_1 -> B1,
	POP_2 -> B2,
	POP_3 -> B3,
	POP_4 -> B4,
	POP_5 -> B5,
	POP_6_SP -> B6,
	POP_7_SP -> B7,
	POP_8_SP -> B8,
	POP_9_SP -> B9,
});

mappings!(Pop9Lane, new_dp, {
	POP_1 -> B1,
	POP_2 -> B2,
	POP_3 -> B3,
	POP_4 -> B4,
	POP_5 -> B5,
	POP_6_DP -> B6,
	POP_7_DP -> B7,
	POP_8_DP -> B8,
	POP_9_DP -> B9,
});

/// What input lane does this channel correspond to?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneKind {
	/// A playable note occurs here.
	Note,
	/// A playable long note occurs here.
	LongNote,
	/// A mine occurs here.
	Mine,
	/// The keysound for this column is changed here.
	KeysoundOnly,
}

/// The playable lanes in a chart. Different [`Mode`]s map these differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::From)]
#[allow(missing_docs)]
pub enum LanePosition {
	Pop5(Pop5Lane),
	Pop9(Pop9Lane),
	Beat5(Beat5Lane),
	Beat10(Beat10Lane),
	Beat7(Beat7Lane),
	Beat14(Beat14Lane),
}

/// A playable lane in a chart, alongside its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane {
	/// Where is this event occuring? i.e. what column?
	pub pos: LanePosition,
	/// What kind of event is occurong in this lane?
	pub kind: LaneKind,
}

impl Lane {
	/// Work out what lane this channel is for.
	///
	/// Returns None if the provided channel is not a lane for this gamemode.
	pub fn new(ch: Channel, mode: Mode) -> Option<Self> {
		let (pos, kind) = match mode {
			Mode::Beat5 => Beat5Lane::new(ch).map(|(a, b)| (a.into(), b)),
			Mode::Beat7 => Beat7Lane::new(ch).map(|(a, b)| (a.into(), b)),
			Mode::Beat10 => Beat10Lane::new(ch).map(|(a, b)| (a.into(), b)),
			Mode::Beat14 => Beat14Lane::new(ch).map(|(a, b)| (a.into(), b)),
			Mode::Pop5SP => Pop5Lane::new_sp(ch).map(|(a, b)| (a.into(), b)),
			Mode::Pop9SP => Pop9Lane::new_sp(ch).map(|(a, b)| (a.into(), b)),
			Mode::Pop5DP => Pop5Lane::new_dp(ch).map(|(a, b)| (a.into(), b)),
			Mode::Pop9DP => Pop9Lane::new_dp(ch).map(|(a, b)| (a.into(), b)),
			Mode::Kb24 => unimplemented!("Kb24 is not supported"),
			Mode::Kb48 => unimplemented!("Kb48 is not supported"),
		}?;

		Some(Self { pos, kind })
	}
}
