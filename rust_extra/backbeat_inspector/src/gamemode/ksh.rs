use rg_formats::ksh::Chart;

use super::RecognisedGamemode;

pub(crate) fn gamemode(_chart: &Chart) -> RecognisedGamemode {
	RecognisedGamemode::Kshoot
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn always_kshoot() {
		let chart = rg_formats::ksh::from_bytes(b"title=t\n--\n0000|00|--\n").unwrap();
		assert_eq!(gamemode(&chart), RecognisedGamemode::Kshoot);
	}
}
