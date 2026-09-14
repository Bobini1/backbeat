use rg_formats::bms::Chart;

use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, text, text_opt};

/// Extract normalized metadata from a parsed BMS-family chart.
pub(crate) fn extract(chart: &Chart) -> ChartExtract {
	let meta = &chart.metadata;

	// Prefer structured fields; fall back to raw tags (banner/stagefile/backbmp
	// are often only present on the raw tag map today).
	let tag_path = |name: &str| chart.get_tag_maybestr(name).and_then(path_string);

	ChartExtract {
		song: SongExtract {
			title: text_opt(meta.title.as_deref()),
			subtitle: text_opt(meta.subtitle.as_deref()),
			artist: text_opt(meta.artist.as_deref()),
			genre: text_opt(meta.genre.as_deref()),
			preview: None,
			title_translit: None,
			artist_translit: None,
		},
		chart: ChartMetaExtract {
			credit: text_opt(meta.subartist.as_deref()),
			charter_note: None,
			difficulty_label: text(chart.get_tag_pretty("DIFFICULTY")),
			level: chart.get_tag_maybestr("PLAYLEVEL").and_then(text),
			chart_name: None,
		},
		assets: AssetPathExtract {
			// BMS is keysounded, not a single song asset.
			music: None,
			banner: meta
				.banner
				.as_deref()
				.and_then(path_string)
				.or_else(|| tag_path("BANNER")),
			background: meta
				.backbmp
				.as_deref()
				.and_then(path_string)
				.or_else(|| tag_path("BACKBMP")),
			jacket: None,
			signature: None,
		},
	}
}
