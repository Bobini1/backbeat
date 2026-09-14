import { A } from "@solidjs/router";
import { For, type JSXElement } from "solid-js";
import { Dynamic } from "solid-js/web";

import styles from "./DetailLister.module.css";

export interface DetailListerProps<T> {
	content: (item: T) => JSXElement;
	detail: (item: T) => JSXElement;
	href?: (item: T) => string;
	items: T[];
}

export default function DetailLister<T>(props: DetailListerProps<T>) {
	return (
		<div class={styles.list}>
			<For each={props.items}>
				{(item) => (
					<li class={styles.container}>
						<Dynamic
							class={styles.item}
							component={props.href ? A : "div"}
							href={props.href?.(item)}
						>
							<div class={styles.main}>
								<div class={styles.content}>{props.content(item)}</div>
								<div class={styles.detail}>{props.detail(item)}</div>
							</div>
						</Dynamic>
					</li>
				)}
			</For>
		</div>
	);
}
