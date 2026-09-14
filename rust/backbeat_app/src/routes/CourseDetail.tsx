import { useParams } from "@solidjs/router";

import { CollectionCoverageDetail } from "../components/collection/CollectionCoverageDetail";
import { BundleStatus } from "../components/collection/CoverageCount";
import { CopyableId } from "../components/CopyableId";
import Lister from "../components/Lister";
import { type CourseContents, downloadCollectionContent, getCourse } from "../lib/tauri";

function summarizeCourse(course: CourseContents) {
	return {
		installed: course.charts.filter((chart) => chart.bundle_id !== null).length,
		total: course.charts.length,
	};
}

export default function CourseDetail() {
	const params = useParams();

	return (
		<CollectionCoverageDetail
			kind="course"
			loadContents={getCourse}
			startDownload={(url, name) => downloadCollectionContent("course", url, name)}
			subtitle="Sequence of charts played back-to-back."
			summarize={summarizeCourse}
			unit="chart"
			urlParam={params.url ?? ""}
		>
			{(data) => (
				<Lister
					content={(bundle) => bundle.desc}
					footer={(bundle) => (
						<>
							<BundleStatus installed={bundle.bundle_id !== null} />
							<CopyableId value={bundle.id} />
						</>
					)}
					href={(bundle) => bundle.bundle_id !== null && `/bundles/${bundle.bundle_id}`}
					items={data.charts}
				/>
			)}
		</CollectionCoverageDetail>
	);
}
