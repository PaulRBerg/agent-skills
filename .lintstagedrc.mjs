/**
 * @type {import("lint-staged").Configuration}
 */
export default {
  "*": "bash -c 'just evm-atlas-check' --",
  "{skills/*/scripts/*.sh,tests/**/*.sh,tests/**/*.bats}": "shellcheck",
  "**/*.{md,json,jsonc,yaml,yml}":
    "bunx --no-install prettier --write --cache --cache-location .cache/prettier/.prettier-cache --log-level warn",
  "{skills/**/{SKILL.md,agents/openai.yaml},README.md}": "bash -c 'just skill-check' --",
};
