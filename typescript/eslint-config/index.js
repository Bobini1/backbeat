import eslint from "@eslint/js";
import pluginImport from "eslint-plugin-import";
import pluginPerfectionist from "eslint-plugin-perfectionist";
import pluginUnusedImports from "eslint-plugin-unused-imports";
import globals from "globals";
import tseslint from "typescript-eslint";

const unusedVarsOptions = {
	args: "all",
	argsIgnorePattern: "^_",
	caughtErrors: "all",
	caughtErrorsIgnorePattern: "^_",
	destructuredArrayIgnorePattern: "^_",
	ignoreRestSiblings: true,
	varsIgnorePattern: "^_",
};

const base = tseslint.config(
	eslint.configs.recommended,
	...tseslint.configs.recommended,
	pluginPerfectionist.configs["recommended-natural"],
	{
		name: "backbeat/base",
		files: ["**/*.{ts,tsx,astro}"],
		plugins: {
			import: pluginImport,
			"unused-imports": pluginUnusedImports,
		},
		languageOptions: {
			ecmaVersion: 2022,
			sourceType: "module",
			parserOptions: {
				project: true,
			},
		},
		rules: {
			...pluginImport.flatConfigs.recommended.rules,
			...pluginImport.flatConfigs.typescript.rules,
			"@typescript-eslint/consistent-type-imports": [
				"warn",
				{
					disallowTypeAnnotations: true,
					fixStyle: "inline-type-imports",
					prefer: "type-imports",
				},
			],
			"@typescript-eslint/explicit-function-return-type": "off",
			"@typescript-eslint/explicit-module-boundary-types": "off",
			"@typescript-eslint/no-non-null-assertion": "off",
			"@typescript-eslint/no-unused-vars": "off",
			"unused-imports/no-unused-imports": "error",
			"unused-imports/no-unused-vars": ["warn", unusedVarsOptions],
			"perfectionist/sort-objects": "off",
			"arrow-body-style": "error",
			curly: "error",
			eqeqeq: "error",
			"import/no-duplicates": ["error", { "prefer-inline": true }],
			"import/no-unresolved": "off",
			"no-var": "error",
			"prefer-const": "error",
			"@typescript-eslint/no-explicit-any": "warn",
		},
		settings: {
			"import/resolver": {
				typescript: {
					alwaysTryTypes: true,
				},
			},
		},
	},
	{
		name: "backbeat/test-files",
		files: ["**/*.test.ts"],
		rules: {
			"no-await-in-loop": "off",
		},
	},
);

const node = {
	name: "backbeat/node",
	files: ["**/*.{js,mjs,cjs,ts,tsx}"],
	languageOptions: {
		globals: {
			...globals.node,
		},
	},
};

export default {
	base,
	node,
};
