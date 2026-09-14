use rg_formats::kson::Kson;

use super::RecognisedGamemode;

pub(crate) fn gamemode(_chart: &Kson) -> RecognisedGamemode {
	RecognisedGamemode::Kshoot
}
