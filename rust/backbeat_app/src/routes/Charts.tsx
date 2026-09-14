import { createEffect, createResource, createSignal, onCleanup, Show } from "solid-js";

import { ExtensionFilter } from "../components/ExtensionFilter";
import Lister from "../components/Lister";
import "../components/search-list.css";
import { PageHeader } from "../components/PageHeader";
import Button from "../components/primitives/Button";
import TextInput from "../components/primitives/TextInput";
import RandomColourBadge from "../components/RandomColourBadge";
import { type ChartEntry, listAvailableExtensions, searchCharts } from "../lib/tauri";

const PAGE_SIZE = 50;
const DEBOUNCE_MS = 250;
const SEARCHING_INDICATOR_DELAY_MS = 200;

export default function Charts() {
	const [query, setQuery] = createSignal("");
	const [debouncedQuery, setDebouncedQuery] = createSignal("");
	const [extension, setExtension] = createSignal("");
	const [offset, setOffset] = createSignal(0);
	const [rows, setRows] = createSignal<ChartEntry[]>([]);
	const [availableExtensions] = createResource(listAvailableExtensions);

	createEffect(() => {
		const value = query();
		const timer = setTimeout(() => {
			if (value === debouncedQuery()) {
				return;
			}
			setDebouncedQuery(value);
			setOffset(0);
			setRows([]);
		}, DEBOUNCE_MS);
		onCleanup(() => clearTimeout(timer));
	});

	const [page] = createResource(
		() => ({
			q: debouncedQuery().trim(),
			offset: offset(),
			extension: extension(),
		}),
		({ q, offset: pageOffset, extension: selectedExtension }) =>
			searchCharts(q || undefined, pageOffset, PAGE_SIZE, selectedExtension || null),
	);

	createEffect(() => {
		const result = page();
		if (!result) {
			return;
		}
		if (offset() === 0) {
			setRows(result.charts);
		} else {
			setRows((prev) => [...prev, ...result.charts]);
		}
	});

	const [showSearching, setShowSearching] = createSignal(false);

	createEffect(() => {
		if (!page.loading) {
			setShowSearching(false);
			return;
		}
		const timer = setTimeout(() => setShowSearching(true), SEARCHING_INDICATOR_DELAY_MS);
		onCleanup(() => clearTimeout(timer));
	});

	const hasMore = () => page()?.has_more ?? false;
	const total = () => page()?.total ?? 0;
	const filtering = () => debouncedQuery().trim().length > 0 || extension().length > 0;
	const searchingIndicator = () => debouncedQuery().trim().length > 0 && showSearching();

	function onExtensionChange(value: string) {
		setExtension(value);
		setOffset(0);
		setRows([]);
	}

	return (
		<>
			<PageHeader subtitle="Browse and search your installed charts." title="Charts" />

			<div class="search-toolbar">
				<TextInput
					label="Search Charts"
					onInput={(e) => setQuery(e.currentTarget.value)}
					type="search"
					value={query()}
				/>
				<ExtensionFilter
					onChange={onExtensionChange}
					options={availableExtensions() ?? []}
					value={extension()}
				/>
			</div>

			<Show when={page.loading && offset() === 0 && !searchingIndicator()}>
				<div class="empty-state">Loading charts…</div>
			</Show>

			<Show when={searchingIndicator()}>
				<p class="page-subtitle">Searching…</p>
			</Show>

			<Show when={page.error}>
				<p class="error">
					{filtering() ? "Search failed" : "Failed to load charts"}: {String(page.error)}
				</p>
			</Show>

			<Show when={!page.loading && !page.error && total() === 0}>
				<div class="empty-state">
					{filtering() ? "No charts match your search." : "No charts installed yet."}
				</div>
			</Show>

			<Show when={rows().length > 0}>
				<p class="search-summary">
					Showing {rows().length} of {total()}{" "}
					{filtering()
						? total() === 1
							? "match"
							: "matches"
						: total() === 1
							? "chart"
							: "charts"}
				</p>
				<Lister
					content={(chart) => chart.description || "(untitled)"}
					detail={(chart) =>
						chart.extension ? (
							<RandomColourBadge content={`.${chart.extension}`} />
						) : undefined
					}
					href={(chart) => `/bundles/${chart.bundle_id}`}
					items={rows()}
				/>
			</Show>

			<Show when={hasMore() && !page.loading}>
				<Button onClick={() => setOffset((n) => n + PAGE_SIZE)} variant="base">
					See more
				</Button>
			</Show>
		</>
	);
}
