import { A, type AnchorProps } from "@solidjs/router";
import { createMemo, type JSX, type ParentProps, splitProps } from "solid-js";
import { Dynamic } from "solid-js/web";

import { cx } from "../../lib/style";
import styles from "./Button.module.css";

interface AsProps {
	/** Uses the SolidJS router `A` component */
	a: AnchorProps;
	button: JSX.HTMLElementTags["button"];
	/**
	 * Declaritive value which provides a standard `<a>` element.
	 * Not particularly useful, but available regardless
	 * */
	HTMLAnchor: JSX.HTMLElementTags["a"];
}

type As = keyof AsProps;

export type ButtonProps<T extends As> = {
	as?: T;
	variant?: "accent" | "base" | "danger" | "success" | "surface" | "warning";
} & ParentProps<AsProps[T]>;

export default function Button<T extends As>(props: ButtonProps<T>) {
	const [filteredProps, restProps] = splitProps(props, ["as", "variant"]);

	const As = createMemo(() => {
		if (filteredProps.as === "a") {
			return A;
		} else if (filteredProps.as === "HTMLAnchor") {
			return "a";
		} else {
			return "button";
		}
	});

	return (
		<Dynamic
			// TS cannot validate incoming props against an unknown target
			// type validation at this layer is not useful anyway
			{...(restProps as object)}
			class={cx(styles.button, restProps.class)}
			component={As()}
			data-variant={filteredProps.variant ?? "accent"}
		/>
	);
}
