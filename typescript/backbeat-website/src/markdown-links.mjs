import path from "node:path";
import { fileURLToPath } from "node:url";

const docsDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../docs");

function webHref(href, sourcePath) {
	if (!sourcePath || !href.match(/\.md(?:#|$)/) || /^(?:[a-z]+:|\/\/|#)/i.test(href)) return href;

	const [pathname, fragment] = href.split("#", 2);
	const target = pathname.startsWith("/")
		? path.resolve(docsDirectory, pathname.slice(1))
		: path.resolve(path.dirname(sourcePath), pathname);
	const relative = path.relative(docsDirectory, target).replace(/\.md$/, "");
	const route = relative === "index" ? "/docs/" : `/docs/${relative}/`;
	return fragment ? `${route}#${fragment}` : route;
}

function visit(node, sourcePath) {
	if (node.type === "element" && node.tagName === "a") {
		const href = node.properties?.href;
		if (typeof href === "string") {
			const rewritten = webHref(href, sourcePath);
			node.properties.href = rewritten;
		}
	}

	for (const child of node.children ?? []) visit(child, sourcePath);
}

export default function rewriteMarkdownLinks() {
	return (tree, file) => visit(tree, file.path);
}
