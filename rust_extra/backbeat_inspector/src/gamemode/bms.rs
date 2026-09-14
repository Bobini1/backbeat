use rg_formats::bms::{Chart, Mode};

use super::RecognisedGamemode;

pub(crate) fn gamemode(chart: &Chart) -> RecognisedGamemode {
	match chart.metadata.mode {
		Mode::Beat5 => RecognisedGamemode::Bms5k,
		Mode::Beat7 => RecognisedGamemode::Bms7k,
		Mode::Beat10 => RecognisedGamemode::Bms10k,
		Mode::Beat14 => RecognisedGamemode::Bms14k,
		Mode::Pop5SP | Mode::Pop5DP => RecognisedGamemode::Pms5b,
		Mode::Pop9SP | Mode::Pop9DP => RecognisedGamemode::Pms9b,
		Mode::Kb24 => RecognisedGamemode::Kms24k,
		Mode::Kb48 => RecognisedGamemode::Kms48k,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn beat7_from_parsed_chart() {
		let chart = rg_formats::bms::from_bytes(
			b"#TITLE Test\n#BPM 120\n#01118:01010101\n",
			"bms",
			rg_formats::bms::BmsRandomStrategy::AlwaysFirstBranch,
		)
		.unwrap();
		assert_eq!(gamemode(&chart), RecognisedGamemode::Bms7k);
	}

	#[test]
	fn defaults_to_5k_without_extend_lanes() {
		let chart = rg_formats::bms::from_bytes(
			b"#TITLE Test\n#BPM 120\n",
			"bms",
			rg_formats::bms::BmsRandomStrategy::AlwaysFirstBranch,
		)
		.unwrap();
		assert_eq!(gamemode(&chart), RecognisedGamemode::Bms5k);
	}
}
