import tsParser from "@typescript-eslint/parser";
import type { ESLint, Linter } from "eslint";
import betterTailwindcss from "eslint-plugin-better-tailwindcss";
import { getDefaultSelectors } from "eslint-plugin-better-tailwindcss/api/defaults";
import reactHooks from "eslint-plugin-react-hooks";

import classNamesPlugin from "./scripts/eslint-class-names.js";

const languageOptions: Linter.LanguageOptions = {
  ecmaVersion: "latest",
  parser: tsParser,
  parserOptions: { ecmaFeatures: { jsx: true } },
  sourceType: "module",
};

const config: Linter.Config[] = [
  { ignores: ["dist/", "node_modules/", "*.d.ts", ".ai/", ".cache/"] },
  {
    files: ["**/*.tsx"],
    languageOptions,
    plugins: { "better-tailwindcss": betterTailwindcss },
    rules: {
      "better-tailwindcss/enforce-canonical-classes": "error",
      "better-tailwindcss/enforce-consistent-class-order": "error",
      "better-tailwindcss/enforce-consistent-line-wrapping": "off",
      "better-tailwindcss/enforce-shorthand-classes": "error",
      "better-tailwindcss/no-conflicting-classes": "error",
      "better-tailwindcss/no-deprecated-classes": "error",
      "better-tailwindcss/no-duplicate-classes": "error",
      "better-tailwindcss/no-unknown-classes": ["error", { detectComponentClasses: true }],
      "better-tailwindcss/no-unnecessary-whitespace": "error",
    },
    settings: {
      "better-tailwindcss": {
        entryPoint: "src/styles.css",
        selectors: [...getDefaultSelectors(), { kind: "attribute", name: ".*ClassName$" }],
      },
    },
  },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions,
    plugins: { local: classNamesPlugin, "react-hooks": reactHooks as unknown as ESLint.Plugin },
    rules: {
      "local/no-classname-concatenation": "error",
      "react-hooks/exhaustive-deps": "error",
      "react-hooks/rules-of-hooks": "error",
    },
  },
];

export default config;
