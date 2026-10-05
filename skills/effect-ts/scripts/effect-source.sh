#!/bin/bash
# Keep a local Effect monorepo checkout on the newest published release, fetching at most once per 24 hours and
# otherwise reusing the cache. Prints `path=<checkout>` and `release=<effect@x.y.z tag>`.
#
# Env: EFFECT_SOURCE_DIR (default ${XDG_CACHE_HOME:-~/.cache}/effect-ts/effect), EFFECT_SOURCE_REPO (clone URL).
set -euo pipefail

dir="${EFFECT_SOURCE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/effect-ts/effect}"
repo="${EFFECT_SOURCE_REPO:-https://github.com/Effect-TS/effect.git}"
max_age=86400
stamp="$dir/.git/effect-source-fetched-at"
now="$(date +%s)"

if [ ! -d "$dir/.git" ]; then
  mkdir -p "$(dirname "$dir")"
  git clone --quiet "$repo" "$dir" >&2
  printf '%s\n' "$now" >"$stamp"
fi

last="$(cat "$stamp" 2>/dev/null || true)"
case "$last" in '' | *[!0-9]*) last=0 ;; esac
if [ $((now - last)) -ge "$max_age" ]; then
  if git -C "$dir" fetch --quiet --tags --force --prune origin >&2; then
    printf '%s\n' "$now" >"$stamp"
  else
    echo "effect-source: fetch failed; using cached source" >&2
  fi
fi

# Every package in a release shares one commit, so the newest effect@* tag on main marks the latest release.
tag="$(git -C "$dir" describe --tags --match 'effect@*' --abbrev=0 origin/main)"
if [ "$(git -C "$dir" rev-parse HEAD)" != "$(git -C "$dir" rev-list -n1 "$tag")" ]; then
  if [ -n "$(git -C "$dir" status --porcelain)" ]; then
    echo "effect-source: $dir has local changes; cannot check out $tag" >&2
    exit 1
  fi
  git -C "$dir" checkout --quiet --detach "$tag"
fi

printf 'path=%s\nrelease=%s\n' "$dir" "$tag"
