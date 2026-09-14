use rg_formats::ssc::SscFile;

use crate::extract::stepmania_preview::preview_start;
use crate::types::{AssetPathExtract, ChartExtract, ChartMetaExtract, SongExtract};
use crate::util::{path_string, tag_string, text, text_opt};

/// Extract normalized metadata from a parsed SSC file.
pub(crate) fn extract(file: &SscFile) -> ChartExtract {
	let song = &file.song;
	let song_msd = |tag: &str| tag_string(song.tags.get_first_value(tag).as_deref());
	let asset = |real_tag: &str, tag: &str| song_msd(real_tag).or_else(|| song_msd(tag));
	let primary = file.charts.first();
	let preview = primary.map(|chart| {
		preview_start(
			&song.tags,
			chart.bpms.as_deref().unwrap_or(&song.bpms),
			chart.stops.as_deref().unwrap_or(&song.stops),
			chart.offset_secs.or(song.offset_secs),
			chart.measures.as_deref().unwrap_or(&[]),
		)
	});

	let (credit, difficulty, level, chart_name) = match primary {
		Some(c) => {
			let chart_msd = |tag: &str| tag_string(c.tags.get_first_value(tag).as_deref());
			(
				chart_msd("CREDIT").or_else(|| text_opt(c.credit.as_deref())),
				c.difficulty.as_ref().map(ToString::to_string),
				c.meter.map(|e| e.to_string()),
				text_opt(c.chart_name.as_deref()).or_else(|| text_opt(c.description.as_deref())),
			)
		}
		None => (None, None, None, None),
	};

	ChartExtract {
		song: SongExtract {
			title: text(&song.title),
			subtitle: text_opt(song.subtitle.as_deref()),
			artist: text(&song.artist),
			genre: song_msd("GENRE"),
			preview,
			title_translit: song_msd("TITLETRANSLIT"),
			artist_translit: song_msd("ARTISTTRANSLIT"),
		},
		chart: ChartMetaExtract {
			credit,
			charter_note: None,
			difficulty_label: difficulty,
			level,
			chart_name,
		},
		assets: AssetPathExtract {
			music: asset("REALMUSIC", "MUSIC")
				.or_else(|| song.music.as_deref().and_then(path_string)),
			banner: asset("REALBANNER", "BANNER"),
			background: asset("REALBACKGROUND", "BACKGROUND"),
			jacket: asset("REALJACKET", "JACKET"),
			signature: asset("REALCDTITLE", "CDTITLE"),
		},
	}
}
