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
import { collectionDocumentPath, describeCollectionError } from "../lib/collectionUi";
import {
	checkInstalledCollectionUpdates,
	type CollectionDocumentStatus,
	listInstalledCollectionDocuments,
} from "../lib/tauri";

const PAGE_SIZE = 50;

export default function Courses() {
	const [courses, { refetch }] = createResource(() => listInstalledCollectionDocuments("course"));
	const [statuses, { refetch: refetchStatuses }] = createResource(
		() => "course" as const,
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
		for (const course of courses() ?? []) {
			modes.add(course.gamemode);
		}
		return [...modes].sort();
	});

	const rows = createMemo(() => {
		const needle = query().trim().toLowerCase();
		const mode = gamemode();
		return (courses() ?? [])
			.filter((course) => {
				if (mode && course.gamemode !== mode) {
					return false;
				}
				if (!needle) {
					return true;
				}
				return [course.name, course.url].some((value) =>
					value.toLowerCase().includes(needle),
				);
			})
			.map((course) => ({
				...course,
				status: statusByUrl().get(course.url),
			}));
	});
	const visibleRows = createMemo(() => rows().slice(0, visibleCount()));

	return (
		<>
			<PageHeader
				actions={
					<Button disabled={courses.loading} onClick={refreshAll} variant="base">
						Refresh
					</Button>
				}
				subtitle="Courses are sequences of charts. You probably know this feature from dan courses."
				title="Courses"
			/>

			<AddCollectionForm currentKind="course" onInstalledHere={refreshAll} />

			<div class="search-toolbar">
				<TextInput
					label="Search courses"
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

			<Show when={courses.loading}>
				<div class="empty-state">Loading courses…</div>
			</Show>

			<Show when={courses.error}>
				<p class="error">
					Failed to load courses: {describeCollectionError(courses.error)}
				</p>
			</Show>
			<Show when={statuses.error}>
				<p class="error">
					Update status unavailable: {describeCollectionError(statuses.error)}
				</p>
			</Show>

			<Show
				fallback={
					<Show when={!courses.loading && !courses.error}>
						<div class="empty-state">No courses installed. Add one above.</div>
					</Show>
				}
				when={courses() && courses()!.length > 0}
			>
				<p class="search-summary">
					Showing {visibleRows().length} of {rows().length} courses
				</p>
				<Show
					fallback={<div class="empty-state">No courses match your search.</div>}
					when={rows().length > 0}
				>
					<Lister
						content={(course) => course.name}
						detail={(course) => <RandomColourBadge content={course.gamemode} />}
						footer={(course) => (
							<>
								<CoverageCount
									installed={course.installed}
									statusDot
									total={course.total}
									unit="chart"
								/>
								<StatusBadge status={course.status} />
							</>
						)}
						href={(course) => collectionDocumentPath("course", course.url)}
						items={visibleRows()}
					/>
				</Show>
				<Show when={visibleRows().length < rows().length && !courses.loading}>
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
