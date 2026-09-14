use std::path::Path;

use backbeat_core::ChartFilename;
use rg_formats::ksh::{Chart, Difficulty};

use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, text};

/// These come with ksm/usc, and aren't real assets
/// we ignore them if specified
const JACKET_PRESETS: &[&str] = &["nowprinting1", "nowprinting2", "nowprinting3"];

/// Extract normalized metadata from a parsed KSH chart.
pub(crate) fn extract(chart: &Chart, filename: &ChartFilename) -> ChartExtract {
	let h = &chart.header;

	let jacket = text(&h.jacket).filter(|j| !JACKET_PRESETS.contains(&j.as_str()));

	ChartExtract {
		song: SongExtract {
			title: text(&h.title),
			subtitle: None,
			artist: text(&h.artist),
			genre: None,
			preview: (h.preview_offset != 0).then_some(u64::from(h.preview_offset)),
			title_translit: text(&h.title_translit),
			artist_translit: text(&h.artist_translit),
		},
		chart: ChartMetaExtract {
			credit: text(&h.effect),
			charter_note: None,
			difficulty_label: Some(difficulty_label(&h.difficulty).to_owned()),
			level: Some(h.level.to_string()),
			chart_name: chart_name(filename),
		},
		assets: AssetPathExtract {
			music: h.audio.first().and_then(path_string),
			banner: None,
			background: None,
			jacket: jacket.and_then(path_string),
			signature: None,
		},
	}
}

fn difficulty_label(difficulty: &Difficulty) -> &'static str {
	match difficulty {
		Difficulty::Light => "Light",
		Difficulty::Challenge => "Challenge",
		Difficulty::Extended => "Extended",
		Difficulty::Infinite => "Infinite",
	}
}

fn chart_name(filename: &ChartFilename) -> Option<String> {
	let chart_name = Path::new(filename.as_str())
		.file_stem()?
		.to_str()?
		.to_owned();
	let lowercase_name = chart_name.to_ascii_lowercase();

	// apply some sdvx corrections
	if [
		"bsc", "adv", "exh", "mxm", "inf", "grv", "hvn", "vvd", "xcd", "nbl", "ult",
	]
	.contains(&lowercase_name.as_str())
	{
		return Some(chart_name.to_uppercase());
	}

	Some(chart_name)
}
