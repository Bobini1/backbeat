import { useParams } from "@solidjs/router";

import { CollectionSettings } from "../components/collection/CollectionSettings";
import { decodeCollectionUrl } from "../lib/collectionUi";

export default function DiffTableSettings() {
	const params = useParams();
	return <CollectionSettings kind="table" url={decodeCollectionUrl(params.url ?? "")} />;
}
