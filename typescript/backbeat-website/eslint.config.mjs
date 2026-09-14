import * as astroParser from "astro-eslint-parser";
import backbeatConfig from "eslint-config-backbeat";
import astroPlugin from "eslint-plugin-astro";

export default [
	{
		ignores: ["node_modules/**", "dist/**"],
	},
	...backbeatConfig.base,
	...astroPlugin.configs.recommended,
	{
		files: ["**/*.astro"],
		languageOptions: {
			parser: astroParser,
		},
		rules: {
			"import/default": "off",
			"import/namespace": "off",
			"import/no-named-as-default": "off",
			"import/no-named-as-default-member": "off",
		},
		settings: {
			"import/parsers": {
				"@typescript-eslint/parser": [".ts", ".tsx"],
				"astro-eslint-parser": [".astro"],
				espree: [".js", ".mjs", ".cjs"],
			},
		},
	},
];
