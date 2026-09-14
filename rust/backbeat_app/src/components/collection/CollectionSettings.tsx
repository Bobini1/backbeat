import { useNavigate } from "@solidjs/router";
import { createResource, createSignal, Show } from "solid-js";

import {
	type CollectionKind,
	collectionKindLabel,
	describeCollectionError,
	pathForCollectionKind,
} from "../../lib/collectionUi";
import { listInstalledCollectionDocuments, uninstallCollectionDocument } from "../../lib/tauri";
import { useConfirmDestructive } from "../../lib/useConfirmDestructive";
import { BackLink } from "../BackLink";
import { ConfirmActions } from "../ConfirmActions";
import { PageHeader } from "../PageHeader";
import Banner from "../primitives/Banner";
import Section from "../primitives/Section";

export function CollectionSettings(props: { kind: CollectionKind; url: string }) {
	const navigate = useNavigate();
	const listPath = () => pathForCollectionKind(props.kind) ?? "/";
	const removeConfirm = useConfirmDestructive();
	const removeDataConfirm = useConfirmDestructive();

	const [document] = createResource(
		() => ({ kind: props.kind, url: props.url }),
		async ({ kind, url }) => {
			const documents = await listInstalledCollectionDocuments(kind);
			return documents.find((entry) => entry.url === url) ?? null;
		},
	);
	const [removing, setRemoving] = createSignal(false);
	const [removeError, setRemoveError] = createSignal<null | string>(null);
	const [removeDataWorking, setRemoveDataWorking] = createSignal(false);

	const kindLabel = () => collectionKindLabel(props.kind).toLowerCase();
	const pageTitle = () => {
		const name = document()?.name;
		const kind = collectionKindLabel(props.kind);
		return name ? `Delete ${kind} ${name}` : `Delete ${kind}`;
	};

	function armRemove() {
		setRemoveError(null);
		removeDataConfirm.cancel();
		removeConfirm.arm();
	}

	async function handleRemove() {
		setRemoving(true);
		setRemoveError(null);
		try {
			await uninstallCollectionDocument(props.url);
			navigate(listPath());
		} catch (error) {
			setRemoveError(describeCollectionError(error));
			setRemoving(false);
			removeConfirm.cancel();
		}
	}

	function armRemoveAndUninstallData() {
		setRemoveError(null);
		removeConfirm.cancel();
		removeDataConfirm.arm();
	}

	async function handleRemoveAndUninstallData() {
		setRemoveDataWorking(true);
		setRemoveError(null);
		try {
			await uninstallCollectionDocument(props.url, true);
			navigate(listPath());
		} catch (error) {
			setRemoveError(describeCollectionError(error));
			setRemoveDataWorking(false);
			removeDataConfirm.cancel();
		}
	}

	return (
		<>
			<p class="breadcrumb">
				<BackLink />
			</p>

			<PageHeader title={pageTitle()} />

			<Show when={document.loading}>
				<Banner>Loading settings…</Banner>
			</Show>
			<Show when={document.error}>
				<Banner variant="danger">
					Failed to load {kindLabel()}: {describeCollectionError(document.error)}
				</Banner>
			</Show>
			<Show when={!document.loading && !document.error && document() === null}>
				<Banner variant="warning">This {kindLabel()} isn't installed.</Banner>
			</Show>

			<Show when={document()}>
				<Section>
					<Banner>
						Remove this {kindLabel()} from your library. Whatever charts you've
						downloaded, stay.
					</Banner>
					<ConfirmActions
						armLabel="Remove"
						confirming={removeConfirm.confirming()}
						confirmLabel="Confirm remove"
						disabled={removeDataWorking()}
						onArm={armRemove}
						onCancel={removeConfirm.cancel}
						onConfirm={handleRemove}
						working={removing()}
						workingLabel="Removing…"
					/>

					<hr class="section-divider" />

					<Banner>
						Or remove the {kindLabel()} and clear out the charts it had installed.
						<br />
						Don't worry, we'll keep charts if they're part of other collections.
					</Banner>
					<ConfirmActions
						armLabel="Remove and uninstall data"
						confirming={removeDataConfirm.confirming()}
						confirmLabel="Confirm uninstall data"
						disabled={removing()}
						onArm={armRemoveAndUninstallData}
						onCancel={removeDataConfirm.cancel}
						onConfirm={handleRemoveAndUninstallData}
						working={removeDataWorking()}
						workingLabel="Uninstalling…"
					/>

					<Show when={removeError()}>
						<Banner variant="danger">{removeError()}</Banner>
					</Show>
				</Section>
			</Show>
		</>
	);
}
