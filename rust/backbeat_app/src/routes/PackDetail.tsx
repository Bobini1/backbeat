import { useParams } from "@solidjs/router";

import { CollectionCoverageDetail } from "../components/collection/CollectionCoverageDetail";
import { BundleStatus } from "../components/collection/CoverageCount";
import { CopyableId } from "../components/CopyableId";
import Lister from "../components/Lister";
import { downloadCollectionContent, getPack, type PackContents } from "../lib/tauri";

function summarizePack(pack: PackContents) {
	return {
		installed: pack.bundles.filter((bundle) => bundle.installed).length,
		total: pack.bundles.length,
	};
}

export default function PackDetail() {
	const params = useParams();

	return (
		<CollectionCoverageDetail
			kind="pack"
			loadContents={getPack}
			startDownload={(url, name) => downloadCollectionContent("pack", url, name)}
			subtitle="Bundles of charts published together."
			summarize={summarizePack}
			unit="chart"
			urlParam={params.url ?? ""}
		>
			{(data) => (
				<Lister
					content={(bundle) => bundle.desc}
					footer={(bundle) => (
						<>
							<BundleStatus installed={bundle.installed} />
							<CopyableId value={bundle.id} />
						</>
					)}
					href={(bundle) => bundle.installed && `/bundles/${bundle.id}`}
					items={data.bundles}
				/>
			)}
		</CollectionCoverageDetail>
	);
}
