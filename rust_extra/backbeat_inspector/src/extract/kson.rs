use rg_formats::kson::{Difficulty, Kson};

use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, text, text_opt};

const JACKET_PRESETS: &[&str] = &["nowprinting1", "nowprinting2", "nowprinting3"];

/// Extract normalized metadata from a parsed KSON chart.
pub(crate) fn extract(chart: &Kson) -> ChartExtract {
	let meta = &chart.meta;

	let jacket = meta
		.jacket_filename
		.as_deref()
		.and_then(path_string)
		.filter(|j| !JACKET_PRESETS.contains(&j.as_str()));

	ChartExtract {
		song: SongExtract {
			title: text(&meta.title),
			subtitle: None,
			artist: text(&meta.artist),
			genre: None,
			preview: chart
				.audio
				.as_ref()
				.and_then(|audio| audio.bgm.as_ref())
				.and_then(|bgm| bgm.preview.as_ref())
				.map(|preview| preview.offset),
			title_translit: text_opt(meta.title_translit.as_deref()),
			artist_translit: text_opt(meta.artist_translit.as_deref()),
		},
		chart: ChartMetaExtract {
			credit: text(&meta.chart_author),
			charter_note: None,
			difficulty_label: Some(difficulty_label(&meta.difficulty)),
			level: Some(meta.level.to_string()),
			chart_name: None,
		},
		assets: AssetPathExtract {
			music: chart
				.audio
				.as_ref()
				.and_then(|a| a.bgm.as_ref())
				.and_then(|b| b.filename.as_deref())
				.and_then(path_string),
			banner: None,
			background: None,
			jacket,
			signature: None,
		},
	}
}

fn difficulty_label(difficulty: &Difficulty) -> String {
	match difficulty {
		Difficulty::Index(0) => "light".to_owned(),
		Difficulty::Index(1) => "challenge".to_owned(),
		Difficulty::Index(2) => "extended".to_owned(),
		Difficulty::Index(_) => "infinite".to_owned(),
		Difficulty::Name(name) => name.clone(),
	}
}
