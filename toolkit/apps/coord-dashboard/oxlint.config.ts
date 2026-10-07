import { defineConfig } from "oxlint";
import core from "ultracite/oxlint/core";

export default defineConfig({
  extends: [core],
  ignorePatterns: ["dist/**", "node_modules/**", ".cache/**", ".ai/**"],
  overrides: [
    {
      files: ["**/*.{ts,tsx}"],
      rules: {
        // TypeScript permits paired schema values and types.
        "no-redeclare": "off",
      },
    },
  ],
  rules: {
    "default-case": "off",
    "func-names": "off",
    "func-style": "off",
    "no-inline-comments": "off",
    "no-use-before-define": "off",
    "oxc/no-barrel-file": ["error", { threshold: 0 }],
    "prefer-destructuring": "off",
    "sort-keys": "off",
    "typescript/consistent-type-definitions": ["error", "type"],
    "unicorn/consistent-function-scoping": "off",
    "unicorn/no-useless-undefined": "off",
    "unicorn/prefer-export-from": "off",
    "unicorn/prefer-single-call": "off",
  },
});
