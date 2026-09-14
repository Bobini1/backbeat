use crate::extract::stepmania_preview::preview_start;
use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, tag_string, text, text_opt};
use rg_formats::sm::Chart;

/// Extract normalized metadata from a parsed SM chart.
pub(crate) fn extract(chart: &Chart) -> ChartExtract {
	let tags = &chart.tags;
	let msd = |tag: &str| tag_string(tags.get_first_value(tag).as_deref());
	let asset = |real_tag: &str, tag: &str| msd(real_tag).or_else(|| msd(tag));

	let credit = msd("CREDIT").or_else(|| {
		let author = String::from_utf8_lossy(&chart.chart_data.author);
		let lower = author.to_ascii_lowercase();
		if author.trim().is_empty() || lower == "copied from" || lower == "blank" {
			None
		} else {
			text(author.as_ref())
		}
	});

	ChartExtract {
		song: SongExtract {
			title: text(&chart.song_info.title),
			subtitle: None,
			artist: text(&chart.song_info.artist),
			genre: msd("GENRE"),
			preview: Some(preview_start(
				&chart.tags,
				&chart.song_info.bpms,
				&chart.song_info.stops,
				chart.song_info.offset_secs,
				&chart.chart_data.notedata,
			)),
			title_translit: msd("TITLETRANSLIT"),
			artist_translit: msd("ARTISTTRANSLIT"),
		},
		chart: ChartMetaExtract {
			credit,
			charter_note: text_opt(chart.song_info.subtitle.as_deref()),
			difficulty_label: Some(chart.chart_data.difficulty.to_string()),
			level: Some(chart.chart_data.level.to_string()),
			chart_name: Some(format!(
				"{} {}",
				chart.chart_data.difficulty, chart.chart_data.level
			)),
		},
		assets: AssetPathExtract {
			// `REAL*` is added on by `bkb enrich`
			// because stepmania has extremely clever code for asset resolution
			music: asset("REALMUSIC", "MUSIC").or_else(|| {
				chart
					.song_info
					.music
					.as_ref()
					.and_then(|p| path_string(p.to_string_lossy()))
			}),
			banner: asset("REALBANNER", "BANNER"),
			background: asset("REALBACKGROUND", "BACKGROUND"),
			jacket: asset("REALJACKET", "JACKET"),
			signature: asset("REALCDTITLE", "CDTITLE"),
		},
	}
}
