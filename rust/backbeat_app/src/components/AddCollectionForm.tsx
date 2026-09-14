import { useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";

import {
	type CollectionKind,
	collectionKindLabel,
	describeCollectionError,
	normalizeCollectionUrl,
	pathForCollectionKind,
} from "../lib/collectionUi";
import { installCollectionDocument } from "../lib/tauri";
import FormInput from "./primitives/FormInput";

export function AddCollectionForm(props: {
	currentKind: CollectionKind;
	onInstalledHere: () => void;
}) {
	const navigate = useNavigate();
	const [url, setUrl] = createSignal("");
	const [adding, setAdding] = createSignal(false);
	const [addError, setAddError] = createSignal<null | string>(null);
	const addLabel = () => `Add new ${collectionKindLabel(props.currentKind).toLowerCase()}`;

	async function handleAdd(event: SubmitEvent) {
		event.preventDefault();
		const corrected = normalizeCollectionUrl(url());
		setUrl(corrected);
		setAdding(true);
		setAddError(null);
		try {
			const kind = await installCollectionDocument(corrected);
			setUrl("");
			if (kind === props.currentKind) {
				props.onInstalledHere();
			} else {
				const path = pathForCollectionKind(kind);
				if (path) {
					navigate(path);
				} else {
					props.onInstalledHere();
				}
			}
		} catch (err) {
			setAddError(describeCollectionError(err, props.currentKind));
		} finally {
			setAdding(false);
		}
	}

	return (
		<>
			<form class="add-remote-form" onSubmit={handleAdd}>
				<FormInput
					button={{
						label: adding() ? "Adding…" : addLabel(),
						disabled: adding(),
					}}
					label="URL"
					onInput={(e) => setUrl(e.currentTarget.value)}
					placeholder="https://collections.backbeat.ac/example"
					required
					type="text"
					value={url()}
				/>
			</form>
			<Show when={addError()}>
				<p class="error">{addError()}</p>
			</Show>
		</>
	);
}
