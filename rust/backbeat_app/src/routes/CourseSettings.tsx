import { useParams } from "@solidjs/router";

import { CollectionSettings } from "../components/collection/CollectionSettings";
import { decodeCollectionUrl } from "../lib/collectionUi";

export default function CourseSettings() {
	const params = useParams();
	return <CollectionSettings kind="course" url={decodeCollectionUrl(params.url ?? "")} />;
}
