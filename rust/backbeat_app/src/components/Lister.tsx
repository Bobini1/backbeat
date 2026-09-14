import { A } from "@solidjs/router";
import { For, type JSX, Show } from "solid-js";
import { Dynamic } from "solid-js/web";

import styles from "./Lister.module.css";

export interface ListerProps<T> {
	content: (item: T) => JSX.Element;
	detail?: (item: T) => JSX.Element;
	footer?: (item: T) => JSX.Element;
	href?: (item: T) => false | null | string | undefined;
	items: T[];
	style?: JSX.CSSProperties;
}

export interface ListerItemProps {
	content: JSX.Element;
	detail?: JSX.Element;
	footer?: JSX.Element;
	href?: false | null | string | undefined;
	style?: JSX.CSSProperties;
}

export default function Lister<T>(props: ListerProps<T>) {
	return (
		<ul class={styles.list} style={props.style}>
			<For each={props.items}>
				{(item) => (
					<ListerItem
						content={props.content(item)}
						detail={props.detail?.(item)}
						footer={props.footer?.(item)}
						href={props.href?.(item)}
					/>
				)}
			</For>
		</ul>
	);
}

export function ListerItem(props: ListerItemProps) {
	return (
		<li class={styles.container}>
			<Dynamic
				class={styles.item}
				component={props.href ? A : "div"}
				{...(props.href ? { href: props.href } : {})}
				style={props.style}
			>
				<div class={styles.main}>
					<div class={styles.content}>{props.content}</div>
					<Show when={props.detail}>
						{(detail) => <div class={styles.detail}>{detail()}</div>}
					</Show>
				</div>
				{props.footer && <div class={styles.footer}>{props.footer}</div>}
			</Dynamic>
		</li>
	);
}
