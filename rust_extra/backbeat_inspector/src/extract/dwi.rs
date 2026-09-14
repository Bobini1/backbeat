use crate::types::ChartExtract;
use crate::util::tag_string;
use rg_formats::sm::Chart;

/// Extract normalized metadata from a DWI chart.
pub(crate) fn extract(chart: &Chart) -> ChartExtract {
	let mut extract = super::sm::extract(chart);
	let tag = |name: &str| tag_string(chart.tags.get_first_value(name).as_deref());

	extract.chart.credit = extract.chart.credit.or_else(|| tag("AUTHOR"));
	extract.chart.chart_name =
		tag("CHARTNAME").or_else(|| Some(chart.chart_data.difficulty.to_string()));
	extract
}
