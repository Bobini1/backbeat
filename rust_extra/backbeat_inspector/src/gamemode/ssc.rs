use rg_formats::ssc::SscChart;

use super::RecognisedGamemode;
use super::sm;

pub(crate) fn gamemode(chart: &SscChart) -> RecognisedGamemode {
	let Some(st) = chart.steps_type.as_ref() else {
		return RecognisedGamemode::Unknown;
	};

	sm::from_steps_type(st)
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::*;

	#[test]
	fn lights_cabinet() {
		let ssc = b"#TITLE:T;\n#BPMS:0.000=120.000;\n#NOTEDATA:;\n#STEPSTYPE:lights-cabinet;\n#DIFFICULTY:Hard;\n#METER:8;\n#NOTES:\n000000\n;\n";
		let file = rg_formats::ssc::from_bytes(ssc, Path::new("chart.ssc"));
		assert_eq!(
			gamemode(&file.charts[0]),
			RecognisedGamemode::SmLightsCabinet
		);
	}
}
