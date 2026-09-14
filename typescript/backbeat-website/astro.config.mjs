// @ts-check
import { unified } from "@astrojs/markdown-remark";
import { defineConfig } from "astro/config";

import remarkCodeTabs, { codeTabsHandler } from "./src/remark-code-tabs.mjs";
import rewriteMarkdownLinks from "./src/markdown-links.mjs";

export default defineConfig({
	markdown: {
		processor: unified({
			remarkPlugins: [remarkCodeTabs],
			rehypePlugins: [rewriteMarkdownLinks],
			remarkRehype: { handlers: { codeTabs: codeTabsHandler } },
		}),
		shikiConfig: {
			themes: { dark: "github-dark", light: "github-light" },
			defaultColor: false,
		},
		syntaxHighlight: "shiki",
	},
	site: "https://backbeat.ac",
});
