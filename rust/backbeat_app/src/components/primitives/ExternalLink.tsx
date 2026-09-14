import { type ParentProps } from "solid-js";

import { cx } from "../../lib/style";
import { openUrl } from "../../lib/tauri";
import styles from "./ExternalLink.module.css";

export interface ExternalLinkProps {
	class?: string;
	href: string;
}

export default function ExternalLink(props: ParentProps<ExternalLinkProps>) {
	return (
		<button
			class={cx(props.class, styles.link)}
			onClick={() => openUrl(props.href)}
			type="button"
		>
			{props.children}
		</button>
	);
}
