import { useParams } from "@solidjs/router";

import { CollectionSettings } from "../components/collection/CollectionSettings";
import { decodeCollectionUrl } from "../lib/collectionUi";

export default function PackSettings() {
	const params = useParams();
	return <CollectionSettings kind="pack" url={decodeCollectionUrl(params.url ?? "")} />;
}
