import { defineConfig } from "oxfmt";
import ultracite from "ultracite/oxfmt";

export default defineConfig({
  ...ultracite,
  ignorePatterns: [
    ...(ultracite.ignorePatterns ?? []),
    "**/*.md",
    "**/*.mdx",
    "**/*.yml",
    "**/*.yaml",
    "dist/**",
    "node_modules/**",
    ".cache/**",
    ".ai/**",
  ],
  printWidth: 100,
  sortPackageJson: false,
  // ESLint sorts classes against this application's Tailwind theme.
  sortTailwindcss: false,
});
