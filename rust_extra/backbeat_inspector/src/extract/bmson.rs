use rg_formats::bmson::Bmson;

use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, text};

/// Extract normalized metadata from a parsed BMSON chart.
pub(crate) fn extract(chart: &Bmson) -> ChartExtract {
	let info = &chart.info;

	ChartExtract {
		song: SongExtract {
			title: text(&info.title),
			subtitle: text(&info.subtitle),
			artist: text(&info.artist),
			genre: text(&info.genre),
			preview: None,
			title_translit: None,
			artist_translit: None,
		},
		chart: ChartMetaExtract {
			credit: info.subartists.first().and_then(|s| text(s.as_str())),
			charter_note: None,
			difficulty_label: text(&info.chart_name),
			level: Some(info.level.to_string()),
			chart_name: text(&info.chart_name),
		},
		assets: AssetPathExtract {
			// BMSON has sound channels, not a single song asset.
			music: None,
			banner: info.banner_image.as_deref().and_then(path_string),
			background: info.back_image.as_deref().and_then(path_string),
			jacket: None,
			signature: None,
		},
	}
}
