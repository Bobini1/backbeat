import { type JSX, Show, splitProps } from "solid-js";

import Button from "./Button";
import styles from "./FormInput.module.css";

export interface InputProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "id"> {
	button?: InputButton;
	label: string;
	type?: "email" | "number" | "password" | "text" | "url";
}

interface InputButton extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
	label: string;
}

/**
 * An input component with a label and an optional submit button all inline
 * Designed to be used within a form element or handle actions programatically
 * */
export default function Input(props: InputProps) {
	const [filteredProps, restProps] = splitProps(props, ["button", "label"]);

	const id = () => `form-input-${filteredProps.label.replace(/\s+/g, "-").toLowerCase()}`;

	return (
		<div class={styles.container}>
			<label class={styles.label} for={id()}>
				{filteredProps.label}
			</label>
			<input id={id()} type="text" {...restProps} />
			<Show when={filteredProps.button}>
				{(button) => {
					const [fp, rest] = splitProps(button(), ["label", "class"]);

					return (
						<Button type="submit" {...rest} class={fp.class}>
							{fp.label}
						</Button>
					);
				}}
			</Show>
		</div>
	);
}
