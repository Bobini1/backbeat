import { useParams } from "@solidjs/router";
import { createSignal, For, onCleanup, onMount } from "solid-js";

import { CollectionCoverageDetail } from "../components/collection/CollectionCoverageDetail";
import Lister from "../components/Lister";
import { ProgressBar } from "../components/ProgressBar";
import {
	downloadCollectionContent,
	getTable,
	type TableContents,
	type TableContentsLevel,
} from "../lib/tauri";
import styles from "./DiffTableDetail.module.css";

function installedCharts(level: TableContentsLevel): number {
	return level.charts.filter((chart) => chart.bundle_id !== null).length;
}

function summarizeTable(table: TableContents) {
	return table.levels.reduce(
		(summary, level) => ({
			installed: summary.installed + installedCharts(level),
			total: summary.total + level.charts.length,
		}),
		{ installed: 0, total: 0 },
	);
}

function TableLevelList(props: { table: TableContents; url: string }) {
	const [levelNameWidth, setLevelNameWidth] = createSignal("max-content");
	let levelNameSizer!: HTMLDivElement;
	let observer: ResizeObserver | undefined;

	onMount(() => {
		observer = new ResizeObserver(([entry]) => {
			setLevelNameWidth(`${entry.contentRect.width}px`);
		});
		observer.observe(levelNameSizer);
	});

	onCleanup(() => observer?.disconnect());

	return (
		<div class={styles.levelList}>
			<div aria-hidden="true" class={styles.levelNameSizer} ref={levelNameSizer}>
				<For each={props.table.levels}>
					{(level) => <span>{`${props.table.symbol}${level.level}`}</span>}
				</For>
			</div>
			<Lister
				content={(level) => (
					<div class={styles.levelRow}>
						<div class={styles.levelName}>
							{props.table.symbol}
							{level.level}
						</div>
						<ProgressBar
							label={`${props.table.symbol}${level.level} charts installed`}
							max={level.charts.length}
							value={installedCharts(level)}
						/>
						<div class={styles.levelCount}>
							{installedCharts(level)}/{level.charts.length}
						</div>
					</div>
				)}
				href={(level) => `/tables/${props.url}/${encodeURIComponent(level.level)}`}
				items={props.table.levels}
				style={{
					"--level-name-width": levelNameWidth(),
					"--level-count-length": `${summarizeTable(props.table).total}`.length,
				}}
			/>
		</div>
	);
}

export default function DiffTableDetail() {
	const params = useParams();

	return (
		<CollectionCoverageDetail
			kind="table"
			loadContents={getTable}
			startDownload={(url, name) => downloadCollectionContent("table", url, name)}
			subtitle="Difficulty levels, assigned to charts."
			summarize={summarizeTable}
			unit="chart"
			urlParam={params.url ?? ""}
		>
			{(data) => <TableLevelList table={data} url={params.url ?? ""} />}
		</CollectionCoverageDetail>
	);
}
