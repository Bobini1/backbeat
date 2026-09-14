import { type JSX, splitProps } from "solid-js";

import { cx } from "../../lib/style";
import styles from "./TextInput.module.css";

export interface TextInputProps
	extends Omit<
		JSX.InputHTMLAttributes<HTMLInputElement>,
		"aria-label" | "aria-labelledby" | "placeholder"
	> {
	label?: string;
	type?: "number" | "search" | "text" | "url";
}

export default function TextInput(props: TextInputProps) {
	const [filteredProps, restProps] = splitProps(props, ["label", "class"]);

	return (
		<input
			aria-label={filteredProps.label}
			class={cx(styles.input, filteredProps.class)}
			placeholder={props.label}
			type="text"
			{...restProps}
		/>
	);
}
