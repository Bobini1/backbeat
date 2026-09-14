import { createResource, createSignal, Show } from "solid-js";

import { PageHeader } from "../components/PageHeader";
import Banner from "../components/primitives/Banner";
import FormInput from "../components/primitives/FormInput";
import RemoteServerList from "../components/RemoteServerList";
import { addRemote, listRemotes } from "../lib/tauri";

export default function Servers() {
	const [remotes, { refetch }] = createResource(listRemotes);
	const [url, setUrl] = createSignal("");
	const [adding, setAdding] = createSignal(false);
	const [addError, setAddError] = createSignal<null | string>(null);

	async function handleAdd(event: SubmitEvent) {
		event.preventDefault();
		setAdding(true);
		setAddError(null);
		try {
			await addRemote(url());
			setUrl("");
			refetch();
		} catch (err) {
			setAddError(String(err));
		} finally {
			setAdding(false);
		}
	}

	return (
		<>
			<PageHeader subtitle="Servers you can download charts from." title="Servers" />

			<form onSubmit={handleAdd}>
				<FormInput
					button={{ disabled: adding(), label: adding() ? "Adding..." : "Add server" }}
					label="Server URL"
					oninput={(e) => setUrl(e.currentTarget.value)}
					placeholder="https://data.makiba.ac"
					required
					type="url"
					value={url()}
				/>
			</form>

			<Show when={addError()}>
				<Banner variant="danger">{addError()}</Banner>
			</Show>

			<Show
				fallback={
					<Show when={!remotes.loading}>
						<Banner>
							No servers configured. Add one above to start pulling charts.
						</Banner>
					</Show>
				}
				when={remotes() && remotes()!.length > 0}
			>
				<RemoteServerList onRemoved={refetch} remotes={remotes()!} />
			</Show>
		</>
	);
}
