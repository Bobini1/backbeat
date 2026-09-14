import { A } from "@solidjs/router";
import { createEffect, createResource, Show } from "solid-js";

import { PageHeader } from "../components/PageHeader";
import Section from "../components/primitives/Section";
import RemoteServerList from "../components/RemoteServerList";
import StatView from "../components/StatView";
import { formatBytes } from "../lib/format";
import { listRemotes, type StoreStats, storeSummary } from "../lib/tauri";

/** Cap how many remotes are listed on the dashboard before linking out to the full page. */
const MAX_ROWS = 5;

let lastStoreStats: StoreStats | undefined;

export default function Home() {
	const [summary] = createResource(storeSummary, { initialValue: lastStoreStats });
	const [remotes] = createResource(listRemotes);

	createEffect(() => {
		const value = summary();
		if (value) {
			lastStoreStats = value;
		}
	});

	return (
		<>
			<PageHeader subtitle="Here's what you've got installed." title="Home" />

			<StatView>
				{(Stat) => (
					<>
						<Stat label="Charts">{summary()?.charts ?? "-"}</Stat>
						<Stat label="Assets">{summary()?.asset_count ?? "-"}</Stat>
						<Stat label="Size On Disk">
							{summary()
								? formatBytes(summary()!.asset_bytes + summary()!.db_bytes)
								: "-"}
						</Stat>
						<Stat label="Packs">{summary()?.packs ?? "-"}</Stat>
						<Stat label="Courses">{summary()?.courses ?? "-"}</Stat>
						<Stat label="Difficulty Tables">{summary()?.tables ?? "-"}</Stat>
					</>
				)}
			</StatView>
			<Show when={summary.error}>
				<p class="error">Failed to load store summary: {String(summary.error)}</p>
			</Show>

			<hr class="section-divider" />

			<Section
				detail={
					<A class="section-link" href="/servers">
						Manage &rarr;
					</A>
				}
				heading="Servers"
			>
				<Show when={remotes.loading}>
					<div class="empty-state empty-state-compact">Loading servers…</div>
				</Show>
				<Show when={remotes() && remotes()!.length === 0}>
					<div class="empty-state empty-state-compact">
						No servers configured. <A href="/servers">Add one</A> to start pulling
						charts.
					</div>
				</Show>
				<Show when={remotes() && remotes()!.length > 0}>
					<RemoteServerList remotes={remotes()!} />
					<Show when={remotes()!.length > MAX_ROWS}>
						<p class="section-overflow-note">
							+{remotes()!.length - MAX_ROWS} more: <A href="/servers">view all</A>
						</p>
					</Show>
				</Show>
			</Section>
		</>
	);
}
