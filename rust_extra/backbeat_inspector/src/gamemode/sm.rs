use rg_formats::sm::{Chart, StepsType};

use super::RecognisedGamemode;

pub(crate) fn gamemode(chart: &Chart) -> RecognisedGamemode {
	from_steps_type(&chart.chart_data.steps_type)
}

pub(super) fn from_steps_type(steps_type: &StepsType) -> RecognisedGamemode {
	let maybe = match steps_type {
		StepsType::DanceSingle => Some(RecognisedGamemode::SmDanceSingle),
		StepsType::DanceDouble => Some(RecognisedGamemode::SmDanceDouble),
		StepsType::DanceCouple => Some(RecognisedGamemode::SmDanceCouple),
		StepsType::DanceSolo => Some(RecognisedGamemode::SmDanceSolo),
		StepsType::DanceThreepanel => None,
		StepsType::DanceRoutine => None,
		StepsType::PumpSingle => Some(RecognisedGamemode::SmPumpSingle),
		StepsType::PumpHalfDouble => Some(RecognisedGamemode::SmPumpHalfdouble),
		StepsType::PumpDouble => Some(RecognisedGamemode::SmPumpDouble),
		StepsType::PumpCouple => Some(RecognisedGamemode::SmPumpCouple),
		StepsType::PumpRoutine => Some(RecognisedGamemode::SmPumpRoutine),
		StepsType::Kb7Single => None,
		StepsType::Ez2Single => None,
		StepsType::Ez2Double => None,
		StepsType::Ez2Real => None,
		StepsType::ParaSingle => None,
		StepsType::Ds3ddxSingle => None,
		StepsType::BmSingle5 => None,
		StepsType::BmVersus5 => None,
		StepsType::BmDouble5 => None,
		StepsType::BmSingle7 => None,
		StepsType::BmVersus7 => None,
		StepsType::BmDouble7 => None,
		StepsType::ManiaxSingle => None,
		StepsType::ManiaxDouble => None,
		StepsType::TechnoSingle4 => None,
		StepsType::TechnoSingle5 => None,
		StepsType::TechnoSingle8 => None,
		StepsType::TechnoDouble4 => None,
		StepsType::TechnoDouble5 => None,
		StepsType::TechnoDouble8 => None,
		StepsType::PnmFive => None,
		StepsType::PnmNine => None,
		StepsType::LightsCabinet => Some(RecognisedGamemode::SmLightsCabinet),
		StepsType::KickboxHuman => None,
		StepsType::KickboxQuadarm => None,
		StepsType::KickboxInsect => None,
		StepsType::KickboxArachnid => None,
		StepsType::Other(_) => None,
	};

	maybe.unwrap_or(RecognisedGamemode::Unknown)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dance_single() {
		let sm = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0.000=120.000;\n#NOTES:\n     dance-single:\n     :\n     Hard:\n     8:\n     0.000,0.000,0.000,0.000,0.000:\n0000\n,\n0000\n;\n";
		let charts = rg_formats::sm::from_bytes(sm, "chart.sm").unwrap();
		assert_eq!(gamemode(&charts[0]), RecognisedGamemode::SmDanceSingle);
	}

	#[test]
	fn bm_and_pnm_steps_types_are_unmapped() {
		assert_eq!(
			from_steps_type(&StepsType::BmSingle5),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::BmVersus5),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::BmDouble5),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::BmSingle7),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::BmVersus7),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::BmDouble7),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::PnmFive),
			RecognisedGamemode::Unknown
		);
		assert_eq!(
			from_steps_type(&StepsType::PnmNine),
			RecognisedGamemode::Unknown
		);
	}

	#[test]
	fn dance_couple() {
		let sm = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0.000=120.000;\n#NOTES:\n     dance-couple:\n     :\n     Hard:\n     8:\n     0.000,0.000,0.000,0.000,0.000:\n0000\n,\n0000\n;\n";
		let charts = rg_formats::sm::from_bytes(sm, "chart.sm").unwrap();
		assert_eq!(gamemode(&charts[0]), RecognisedGamemode::SmDanceCouple);
	}

	#[test]
	fn lights_cabinet() {
		let sm = b"#TITLE:T;\n#BPMS:0.000=120.000;\n#NOTES:\n     lights-cabinet:\n     :\n     Hard:\n     8:\n     0.000:\n000000\n;\n";
		let charts = rg_formats::sm::from_bytes(sm, "chart.sm").unwrap();
		assert_eq!(gamemode(&charts[0]), RecognisedGamemode::SmLightsCabinet);
	}

	#[test]
	fn pump_couple() {
		let sm = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0.000=120.000;\n#NOTES:\n     pump-couple:\n     :\n     Hard:\n     8:\n     00000:\n00000\n,\n00000\n;\n";
		let charts = rg_formats::sm::from_bytes(sm, "chart.sm").unwrap();
		assert_eq!(gamemode(&charts[0]), RecognisedGamemode::SmPumpCouple);
	}

	#[test]
	fn pump_routine() {
		let sm = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0.000=120.000;\n#NOTES:\n     pump-routine:\n     :\n     Hard:\n     8:\n     00000:\n00000\n,\n00000\n;\n";
		let charts = rg_formats::sm::from_bytes(sm, "chart.sm").unwrap();
		assert_eq!(gamemode(&charts[0]), RecognisedGamemode::SmPumpRoutine);
	}
}
