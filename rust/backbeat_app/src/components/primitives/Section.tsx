import { type JSX, type ParentProps, Show, splitProps } from "solid-js";

import NiceLabel from "./NiceLabel";
import styles from "./Section.module.css";

export interface SectionProps extends ParentProps<JSX.HTMLAttributes<HTMLElement>> {
	/** An element that sits across from the heading */
	detail?: JSX.Element;
	heading?: string;
	headingLevel?: "h2" | "h3" | "h4";
}

/** Creates a `<section>` with flex column styles and optional stylised header */
export default function Section(props: SectionProps) {
	const [sectionProps, divProps] = splitProps(props, ["heading", "headingLevel", "detail"]);

	const idString = () => props.id ?? sectionProps.heading?.toLowerCase().replace(/\s+/g, "-");
	const sectionId = () => idString() && `section-${idString()}`;
	const headingId = () => idString() && `section-heading-${idString()}`;

	return (
		<section {...divProps} aria-labelledby={headingId()} id={sectionId()}>
			<Show when={sectionProps.heading}>
				{(heading) => (
					<header class={styles.header}>
						<NiceLabel as={props.headingLevel ?? "h2"} id={headingId()}>
							{heading()}
						</NiceLabel>
						{sectionProps.detail}
					</header>
				)}
			</Show>
			<div class={styles.content}>{props.children}</div>
		</section>
	);
}
