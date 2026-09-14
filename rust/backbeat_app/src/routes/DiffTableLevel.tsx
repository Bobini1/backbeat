import { useParams } from "@solidjs/router";
import { createMemo, createResource, onMount, Show } from "solid-js";

import { BackLink } from "../components/BackLink";
import { BundleStatus } from "../components/collection/CoverageCount";
import { CopyableId } from "../components/CopyableId";
import Lister from "../components/Lister";
import { PageHeader } from "../components/PageHeader";
import { decodeCollectionUrl } from "../lib/collectionUi";
import { getTable, onCollectionDownloadEvent } from "../lib/tauri";

export function DiffTableLevel() {
	const params = useParams();
	const url = () => decodeCollectionUrl(params.url!);

	const [table, { refetch }] = createResource(url, getTable);
	const levelString = decodeURIComponent(params.level!);

	const level = createMemo(() => table()?.levels.find((l) => l.level === levelString));

	onMount(() => {
		void onCollectionDownloadEvent((event) => {
			if (event.url !== url()) {
				return;
			}

			if (event.state !== "downloading") {
				refetch();
			}
		});
	});

	return (
		<>
			<p class="breadcrumb">
				<BackLink />
			</p>

			<PageHeader
				subtitle={`Charts in the ${table()?.symbol}${levelString} folder.`}
				title={`${table()?.name}: ${table()?.symbol}${levelString}`}
			/>

			<Show when={level()}>
				{(level) => (
					<Lister
						content={(chart) => chart.desc}
						footer={(chart) => (
							<>
								<BundleStatus installed={Boolean(chart.bundle_id)} />
								<CopyableId value={chart.id} />
							</>
						)}
						href={(chart) => chart.bundle_id && `/bundles/${chart.bundle_id}`}
						items={level().charts}
					/>
				)}
			</Show>
		</>
	);
}
