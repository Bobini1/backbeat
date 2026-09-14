import styles from "./RandomColourBadge.module.css";

function generateHue(extension: string): number {
	let hash = 2_166_136_261;
	for (let index = 0; index < extension.length; index += 1) {
		hash ^= extension.charCodeAt(index);
		hash = Math.imul(hash, 16_777_619);
	}
	return (hash >>> 0) % 360;
}

export default function RandomColourBadge(props: { content: string }) {
	const hue = () => generateHue(props.content);

	return (
		<span class={styles.badge} style={`--badge-hue: ${hue()}`}>
			{props.content}
		</span>
	);
}
