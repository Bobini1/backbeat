import { createEffect, createMemo, createResource, createSignal, Show } from "solid-js";

import { AddCollectionForm } from "../components/AddCollectionForm";
import CoverageCount from "../components/collection/CoverageCount";
import StatusBadge from "../components/collection/StatusBadge";
import { GamemodeFilter } from "../components/GamemodeFilter";
import Lister from "../components/Lister";
import { PageHeader } from "../components/PageHeader";
import Button from "../components/primitives/Button";
import TextInput from "../components/primitives/TextInput";
import "../components/search-list.css";
import RandomColourBadge from "../components/RandomColourBadge";
import { collectionDocumentPath } from "../lib/collectionUi";
import {
	checkInstalledCollectionUpdates,
	type CollectionDocumentStatus,
	listInstalledCollectionDocuments,
} from "../lib/tauri";

const PAGE_SIZE = 50;

export default function DiffTables() {
	const [tables, { refetch }] = createResource(() => listInstalledCollectionDocuments("table"));
	const [statuses, { refetch: refetchStatuses }] = createResource(
		() => "table" as const,
		(kind) => checkInstalledCollectionUpdates(kind),
	);
	const [query, setQuery] = createSignal("");
	const [gamemode, setGamemode] = createSignal("");
	const [visibleCount, setVisibleCount] = createSignal(PAGE_SIZE);

	createEffect(() => {
		query();
		gamemode();
		setVisibleCount(PAGE_SIZE);
	});
	const statusByUrl = createMemo(() => {
		const map = new Map<string, CollectionDocumentStatus>();
		for (const status of statuses() ?? []) {
			map.set(status.url, status);
		}
		return map;
	});

	function refreshAll() {
		setVisibleCount(PAGE_SIZE);
		refetch();
		refetchStatuses();
	}

	const gamemodeOptions = createMemo(() => {
		const modes = new Set<string>();
		for (const table of tables() ?? []) {
			modes.add(table.gamemode);
		}
		return [...modes].sort();
	});

	const rows = createMemo(() => {
		const needle = query().trim().toLowerCase();
		const mode = gamemode();
		return (tables() ?? [])
			.filter((table) => {
				if (mode && table.gamemode !== mode) {
					return false;
				}
				if (!needle) {
					return true;
				}
				return [table.name, table.url].some((value) =>
					value.toLowerCase().includes(needle),
				);
			})
			.map((table) => ({
				...table,
				status: statusByUrl().get(table.url),
			}));
	});
	const visibleRows = createMemo(() => rows().slice(0, visibleCount()));

	return (
		<>
			<PageHeader
				actions={
					<Button disabled={tables.loading} onClick={refreshAll} variant="base">
						Refresh
					</Button>
				}
				subtitle="Difficulty Tables assign level information to charts. They're how you know how difficult a chart is."
				title="Difficulty Tables"
			/>

			<AddCollectionForm currentKind="table" onInstalledHere={refreshAll} />

			<div class="search-toolbar">
				<TextInput
					label="Search tables"
					onInput={(e) => setQuery(e.currentTarget.value)}
					type="search"
					value={query()}
				/>
				<GamemodeFilter
					onChange={setGamemode}
					options={gamemodeOptions()}
					value={gamemode()}
				/>
			</div>
			<Show when={tables.loading}>
				<div class="empty-state">Loading tables…</div>
			</Show>
			<Show when={tables.error}>
				<p class="error">Failed to load tables: {String(tables.error)}</p>
			</Show>
			<Show when={statuses.error}>
				<p class="error">Update status unavailable: {String(statuses.error)}</p>
			</Show>
			<Show
				fallback={
					<Show when={!tables.loading && !tables.error}>
						<div class="empty-state">No tables installed. Add one above.</div>
					</Show>
				}
				when={tables() && tables()!.length > 0}
			>
				<p class="search-summary">
					Showing {visibleRows().length} of {rows().length} tables
				</p>
				<Show
					fallback={<div class="empty-state">No tables match your search.</div>}
					when={rows().length > 0}
				>
					<Lister
						content={(table) => table.name}
						detail={(table) => <RandomColourBadge content={table.gamemode} />}
						footer={(table) => (
							<>
								<CoverageCount
									installed={table.installed}
									statusDot
									total={table.total}
									unit="chart"
								/>
								<StatusBadge status={table.status} />
							</>
						)}
						href={(table) => collectionDocumentPath("table", table.url)}
						items={visibleRows()}
					/>
				</Show>
				<Show when={visibleRows().length < rows().length && !tables.loading}>
					<Button
						onClick={() => setVisibleCount((count) => count + PAGE_SIZE)}
						variant="base"
					>
						See more
					</Button>
				</Show>
			</Show>
		</>
	);
}
