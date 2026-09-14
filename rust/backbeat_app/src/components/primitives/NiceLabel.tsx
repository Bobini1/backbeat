import { type JSX, type ParentProps, splitProps } from "solid-js";
import { Dynamic } from "solid-js/web";

import { cx } from "../../lib/style";
import styles from "./NiceLabel.module.css";

type As = "div" | "h1" | "h2" | "h3" | "h4" | "label" | "span";

export type NiceLabelProps<T extends As> = {
	as?: T;
} & ParentProps<JSX.HTMLElementTags[T]>;

export default function SectionHeading<T extends As>(props: NiceLabelProps<T>) {
	const [filteredProps, restProps] = splitProps(props, ["as"]);

	return (
		<Dynamic
			{...(restProps as object)}
			class={cx(styles.heading, restProps.class)}
			component={filteredProps.as ?? "div"}
		/>
	);
}
