/** @type {import("lint-staged").Configuration} */
export default {
  "*.{css,js,jsx,mjs,cjs,json,jsonc,html,ts,tsx,mts,cts}": "just ox-write",
  "*.{md,mdx,yml,yaml}": "just prettier-write",
  "*.{ts,tsx}": "just eslint-check",
};
