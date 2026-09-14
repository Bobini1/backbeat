import { type LucideProps, Package, Route, Scale } from "lucide-solid";
import { createSignal, For, type JSX, Show, splitProps } from "solid-js";

import type { BundleDetailResponse } from "../lib/tauri";

import { collectionDocumentPath, type CollectionKind } from "../lib/collectionUi";
import styles from "./ChartAppearance.module.css";
import Lister from "./Lister";
import NiceLabel from "./primitives/NiceLabel";

export interface ChartAppearanceProps {
	chart: BundleDetailResponse;
}

interface TabBarProps {
	active: CollectionKind;
	chart: BundleDetailResponse;
	setActive: (kind: CollectionKind) => void;
}

export default function ChartAppearance(props: ChartAppearanceProps) {
	const [activeKind, setActiveKind] = createSignal<CollectionKind>("pack");

	const activeItems = () => getAppearances(props.chart, activeKind());

	return (
		<div class={styles.main}>
			<TabBar active={activeKind()} chart={props.chart} setActive={setActiveKind} />
			<Show
				fallback={
					<div class={styles.fallback}>{`Not in any installed ${activeKind()}.`}</div>
				}
				when={activeItems().length > 0}
			>
				<Lister
					content={(appearance) => appearance.name}
					detail={(appearance) =>
						appearance.level ? `${appearance.symbol ?? ""}${appearance.level}` : null
					}
					href={(appearance) =>
						collectionDocumentPath(appearance.collection_kind, appearance.url)
					}
					items={activeItems()}
				/>
			</Show>
		</div>
	);
}

function TabBar(props: TabBarProps) {
	const tabs: CollectionKind[] = ["pack", "table", "course"];

	return (
		<ul class={styles.tabBar} role="tablist">
			<For each={tabs}>
				{(kind) => (
					<Tab
						active={props.active === kind}
						count={getAppearances(props.chart, kind).length}
						kind={kind}
						onClick={() => props.setActive(kind)}
					/>
				)}
			</For>
		</ul>
	);
}

function Tab(
	props: {
		active: boolean;
		count: number;
		kind: CollectionKind;
	} & Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "aria-selected" | "class" | "role">,
) {
	const [tabProps, restProps] = splitProps(props, ["kind"]);

	const Icon = (props: LucideProps) => {
		switch (tabProps.kind) {
			case "course":
				return <Route {...props} />;
			case "pack":
				return <Package {...props} />;
			case "table":
				return <Scale {...props} />;
		}
	};

	return (
		<li class={styles.tabContainer}>
			<button {...restProps} aria-selected={props.active} class={styles.tab} role="tab">
				<Icon aria-hidden size={14} />
				<NiceLabel as="span">{`${tabProps.kind}s`}</NiceLabel>
				<span class={styles.count}>{props.count}</span>
			</button>
		</li>
	);
}

function getAppearances(chart: BundleDetailResponse, kind: CollectionKind) {
	return chart.appearances.filter((appearance) => appearance.collection_kind === kind);
}
