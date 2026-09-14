mod bms;
mod bmson;
mod ksh;
mod kson;
mod sm;
mod ssc;
mod r#type;

pub use self::r#type::RecognisedGamemode;

use crate::parsed::ParsedChart;

pub(crate) fn infer(chart: &ParsedChart) -> RecognisedGamemode {
	match chart {
		ParsedChart::Bms(chart) => bms::gamemode(chart),
		ParsedChart::Bmson(chart) => bmson::gamemode(chart),
		ParsedChart::Sm(chart) | ParsedChart::Dwi(chart) => sm::gamemode(chart),
		ParsedChart::Ssc(file) => match file.charts.as_slice() {
			[chart] => ssc::gamemode(chart),
			_ => RecognisedGamemode::Unknown,
		},
		ParsedChart::Ksh(chart) => ksh::gamemode(chart),
		ParsedChart::Kson(chart) => kson::gamemode(chart),
	}
}
