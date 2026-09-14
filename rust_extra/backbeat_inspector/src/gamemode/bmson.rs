use rg_formats::bmson::Bmson;

use super::RecognisedGamemode;

pub(crate) fn gamemode(chart: &Bmson) -> RecognisedGamemode {
	match chart.info.mode_hint.as_str() {
		"beat-5k" => RecognisedGamemode::Bms5k,
		"beat-7k" => RecognisedGamemode::Bms7k,
		"beat-10k" => RecognisedGamemode::Bms10k,
		"beat-14k" => RecognisedGamemode::Bms14k,
		"popn-5k" => RecognisedGamemode::Pms5b,
		"popn-9k" => RecognisedGamemode::Pms9b,
		"keyboard-24k" => RecognisedGamemode::Kms24k,
		"keyboard-24k-double" => RecognisedGamemode::Kms48k,
		_ => RecognisedGamemode::Unknown,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn beat_7k_mode_hint() {
		let json = br#"{
			"version":"1.0.0",
			"info":{"title":"t","artist":"a","genre":"g","mode_hint":"beat-7k","chart_name":"","level":1,"init_bpm":120.0},
			"sound_channels":[]
		}"#;
		let chart = rg_formats::bmson::from_bytes(json).unwrap();
		assert_eq!(gamemode(&chart), RecognisedGamemode::Bms7k);
	}
}
