export type Variant = "accent" | "base" | "danger" | "success" | "surface" | "warning";

export function cx(...args: unknown[]) {
	return args
		.flat()
		.filter((x) => typeof x === "string")
		.join(" ")
		.trim();
}
