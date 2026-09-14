import { type ComponentProps, createResource, createSignal, type Resource, Show } from "solid-js";

import {
	openUrl,
	pingRemote,
	type PingResult,
	removeRemote,
	type ServerConfig,
} from "../lib/tauri";
import { useConfirmDestructive } from "../lib/useConfirmDestructive";
import { ConfirmActions } from "./ConfirmActions";
import Lister from "./Lister";
import Button from "./primitives/Button";
import ExternalLink from "./primitives/ExternalLink";
import styles from "./RemoteServerList.module.css";
import StatusDot from "./StatusDot";

export interface RemoteServerListProps {
	onRemoved?: () => void;
	remotes: ServerConfig[];
}

interface RemoteRowProps {
	onRemoved?: () => void;
	remote: ServerConfig;
}

interface RemoteConnectionProps {
	status: Resource<PingResult>;
}

export default function RemoteServerList(props: RemoteServerListProps) {
	return (
		<Lister
			content={(item) => <RemoteRow onRemoved={props.onRemoved} remote={item} />}
			items={props.remotes}
		/>
	);
}

function RemoteRow(props: RemoteRowProps) {
	const [status] = createResource(() => props.remote.url, pingRemote);
	const [removing, setRemoving] = createSignal(false);
	const [removeError, setRemoveError] = createSignal<null | string>(null);
	const confirm = useConfirmDestructive();

	const browseContentButton = (
		<Button onClick={() => void openUrl(props.remote.url)} variant="base">
			Browse Content
		</Button>
	);

	const dotState = (): ComponentProps<typeof StatusDot>["status"] => {
		const result = status();
		if (status.loading || result === undefined) {
			return "base";
		}
		return result.up ? "success" : "danger";
	};

	function armConfirm() {
		setRemoveError(null);
		confirm.arm();
	}

	async function handleRemove() {
		if (!props.onRemoved) {
			return;
		}

		setRemoving(true);
		setRemoveError(null);
		try {
			await removeRemote(props.remote.url);
			props.onRemoved();
		} catch (err) {
			setRemoveError(String(err));
			setRemoving(false);
			confirm.cancel();
		}
	}

	return (
		<div class={styles.row}>
			<div class={styles.status}>
				<StatusDot status={dotState()} />
				<div>
					<ExternalLink href={props.remote.url}>{props.remote.url}</ExternalLink>
					<Show fallback={<RemoteConnection status={status} />} when={removeError()}>
						{(error) => <div class={styles.error}>{error()}</div>}
					</Show>
				</div>
			</div>
			<Show fallback={browseContentButton} when={props.onRemoved}>
				<ConfirmActions
					armLabel="Remove"
					confirming={confirm.confirming()}
					confirmLabel="Confirm remove"
					idle={browseContentButton}
					onArm={armConfirm}
					onCancel={confirm.cancel}
					onConfirm={handleRemove}
					working={removing()}
					workingLabel="Removing…"
				/>
			</Show>
		</div>
	);
}

function RemoteConnection(props: RemoteConnectionProps) {
	return (
		<div class={styles.connection} data-error={Boolean(props.status?.()?.error)}>
			<Show
				fallback={props.status.loading ? "Pinging..." : "Waiting..."}
				when={props.status()}
			>
				{(status) =>
					status().up
						? `${status().name ?? "(unnamed server)"} · ${status()?.latency_ms}ms`
						: `Unreachable: ${status()?.error}`
				}
			</Show>
		</div>
	);
}
