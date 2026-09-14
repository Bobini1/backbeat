import { type JSX, Show } from "solid-js";

import "./ConfirmActions.css";
import Button from "./primitives/Button";

export function ConfirmActions(props: {
	armLabel: string;
	confirming: boolean;
	confirmLabel: string;
	disabled?: boolean;
	idle?: JSX.Element;
	onArm: () => void;
	onCancel: () => void;
	onConfirm: () => void;
	working?: boolean;
	workingLabel: string;
}) {
	return (
		<div class="confirm-actions">
			<Show
				fallback={
					<>
						{props.idle}
						<Button
							disabled={props.disabled || props.working}
							onClick={props.onArm}
							variant="danger"
						>
							{props.armLabel}
						</Button>
					</>
				}
				when={props.confirming}
			>
				<Button
					disabled={props.working}
					onClick={props.onCancel}
					type="button"
					variant="base"
				>
					Cancel
				</Button>
				<Button
					disabled={props.working}
					onClick={props.onConfirm}
					type="button"
					variant="danger"
				>
					{props.working ? props.workingLabel : props.confirmLabel}
				</Button>
			</Show>
		</div>
	);
}
