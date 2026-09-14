import { createEffect, createMemo, createResource, createSignal, Show } from "solid-js";

import { AddCollectionForm } from "../components/AddCollectionForm";
import CoverageCount from "../components/collection/CoverageCount";
import { GamemodeFilter } from "../components/GamemodeFilter";
import Lister from "../components/Lister";
import "../components/search-list.css";
import { PageHeader } from "../components/PageHeader";
import Button from "../components/primitives/Button";
import TextInput from "../components/primitives/TextInput";
import RandomColourBadge from "../components/RandomColourBadge";
import { collectionDocumentPath } from "../lib/collectionUi";
import { listInstalledCollectionDocuments } from "../lib/tauri";

const PAGE_SIZE = 50;

export default function Packs() {
	const [packs, { refetch }] = createResource(() => listInstalledCollectionDocuments("pack"));
	const [query, setQuery] = createSignal("");
	const [gamemode, setGamemode] = createSignal("");
	const [visibleCount, setVisibleCount] = createSignal(PAGE_SIZE);

	createEffect(() => {
		query();
		gamemode();
		setVisibleCount(PAGE_SIZE);
	});

	const gamemodeOptions = createMemo(() => {
		const modes = new Set<string>();
		for (const pack of packs() ?? []) {
			modes.add(pack.gamemode);
		}
		return [...modes].sort();
	});

	const rows = createMemo(() => {
		const needle = query().trim().toLowerCase();
		const mode = gamemode();
		return (packs() ?? []).filter((pack) => {
			if (mode && pack.gamemode !== mode) {
				return false;
			}
			if (!needle) {
				return true;
			}
			return [pack.name, pack.url].some((value) => value.toLowerCase().includes(needle));
		});
	});
	const visibleRows = createMemo(() => rows().slice(0, visibleCount()));

	function refreshAll() {
		setVisibleCount(PAGE_SIZE);
		refetch();
	}

	return (
		<>
			<PageHeader
				actions={
					<Button disabled={packs.loading} onClick={refreshAll} variant="base">
						Refresh
					</Button>
				}
				subtitle="Packs are groups of charts. They're like albums, but for charts."
				title="Packs"
			/>

			<AddCollectionForm currentKind="pack" onInstalledHere={refreshAll} />

			<div class="search-toolbar">
				<TextInput
					label="Search packs"
					onInput={(event) => setQuery(event.currentTarget.value)}
					type="search"
					value={query()}
				/>
				<GamemodeFilter
					onChange={setGamemode}
					options={gamemodeOptions()}
					value={gamemode()}
				/>
			</div>
			<Show when={packs.loading}>
				<div class="empty-state">Loading packs…</div>
			</Show>
			<Show when={packs.error}>
				<p class="error">Failed to load packs: {String(packs.error)}</p>
			</Show>
			<Show
				fallback={
					<Show when={!packs.loading && !packs.error}>
						<div class="empty-state">No packs installed. Add one above.</div>
					</Show>
				}
				when={packs() && packs()!.length > 0}
			>
				<p class="search-summary">
					Showing {visibleRows().length} of {rows().length} packs
				</p>
				<Show
					fallback={<div class="empty-state">No packs match your search.</div>}
					when={rows().length > 0}
				>
					<Lister
						content={(pack) => pack.name}
						detail={(pack) => <RandomColourBadge content={pack.gamemode} />}
						footer={(pack) => (
							<CoverageCount
								installed={pack.installed}
								statusDot
								total={pack.total}
								unit="chart"
							/>
						)}
						href={(pack) => collectionDocumentPath("pack", pack.url)}
						items={visibleRows()}
					/>
				</Show>
				<Show when={visibleRows().length < rows().length && !packs.loading}>
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
