#!/bin/bash

set -euo pipefail

fresh_limit_seconds=86400
stale_limit_seconds=604800
lock_wait_seconds=35
stale_lock_seconds=60

usage() {
  cat >&2 <<'EOF'
Usage: fetch-guidance.sh [--refresh] <gpt-6-astra|claude-fable-5-1>

Reuse fresh cached prompting guides and revalidate older fixed official artifacts.
Prints the absolute cached file path on stdout.
EOF
}

die() {
  printf 'agents-brain: %s\n' "$1" >&2
  exit "${2:-1}"
}

refresh=false
if [ "${1:-}" = '--refresh' ]; then
  refresh=true
  shift
fi

if [ "$#" -ne 1 ]; then
  usage
  exit 64
fi

artifact=$1
case "$artifact" in
  gpt-6-astra)
    source_url='https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md'
    body_name='gpt-6-astra-prompting.md'
    content_marker='# Using GPT-6 Astra'
    ;;
  claude-fable-5-1)
    source_url='https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1.md'
    body_name='claude-fable-5-1-prompting.md'
    content_marker='title: Prompting Claude Fable 5.1'
    ;;
  *)
    die "unknown artifact '$artifact'" 64
    ;;
esac

umask 077

if [ -n "${AGENTS_BRAIN_CACHE_DIR:-}" ]; then
  cache_root=$AGENTS_BRAIN_CACHE_DIR
elif [ -n "${XDG_CACHE_HOME:-}" ]; then
  cache_root=$XDG_CACHE_HOME/agents-brain
elif [ "$(uname -s)" = 'Darwin' ]; then
  [ -n "${HOME:-}" ] || die 'HOME is required to resolve the cache directory'
  cache_root=$HOME/Library/Caches/agents-brain
else
  [ -n "${HOME:-}" ] || die 'HOME is required to resolve the cache directory'
  cache_root=$HOME/.cache/agents-brain
fi

mkdir -p "$cache_root" || die "cannot create cache directory: $cache_root"
cache_root=$(cd "$cache_root" && pwd -P) || die "cannot resolve cache directory: $cache_root"

body_file=$cache_root/$body_name
metadata_file=$cache_root/$artifact.meta
lock_dir=$cache_root/$artifact.lock

headers_tmp=''
body_tmp=''
metadata_tmp=''
lock_owned=false

cleanup() {
  if [ -n "$headers_tmp" ]; then
    rm -f "$headers_tmp"
  fi
  if [ -n "$body_tmp" ]; then
    rm -f "$body_tmp"
  fi
  if [ -n "$metadata_tmp" ]; then
    rm -f "$metadata_tmp"
  fi
  if [ "$lock_owned" = true ]; then
    rmdir "$lock_dir" 2>/dev/null || true
  fi
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

lock_attempt=0
while ! mkdir "$lock_dir" 2>/dev/null; do
  now_epoch=$(date -u +%s)
  lock_epoch=$(stat -f '%m' "$lock_dir" 2>/dev/null || stat -c '%Y' "$lock_dir" 2>/dev/null || printf '0\n')
  case "$lock_epoch" in
    ''|*[!0-9]*) lock_epoch=0 ;;
  esac

  if [ "$lock_epoch" -gt 0 ] && [ $((now_epoch - lock_epoch)) -gt "$stale_lock_seconds" ]; then
    if rmdir "$lock_dir" 2>/dev/null; then
      continue
    fi
  fi

  lock_attempt=$((lock_attempt + 1))
  if [ "$lock_attempt" -ge "$lock_wait_seconds" ]; then
    die "cache lock remained busy for $lock_wait_seconds seconds: $artifact"
  fi
  sleep 1
done
lock_owned=true

metadata_value() {
  local key=$1
  local file=$2

  [ -f "$file" ] || return 1
  sed -n "s/^${key}=//p" "$file" | sed -n '1p'
}

validate_body() {
  local file=$1
  local byte_count

  [ -f "$file" ] || return 1
  byte_count=$(wc -c <"$file" | tr -d '[:space:]')
  case "$byte_count" in
    ''|*[!0-9]*) return 1 ;;
  esac
  [ "$byte_count" -ge 1024 ] || return 1
  grep -Fq -- "$content_marker" "$file"
}

allowed_effective_url() {
  [ "$1" = "$source_url" ]
}

final_header_value() {
  local header_name=$1
  local file=$2

  awk -v wanted="$header_name" '
    /^HTTP\// { value = "" }
    {
      line = $0
      sub(/\r$/, "", line)
      separator = index(line, ":")
      if (separator > 0 && tolower(substr(line, 1, separator - 1)) == tolower(wanted)) {
        value = substr(line, separator + 1)
        sub(/^[[:space:]]+/, "", value)
      }
    }
    END { print value }
  ' "$file"
}

write_metadata() {
  local effective_url=$1
  local etag=$2
  local last_modified=$3
  local validated_epoch=$4
  local validated_utc=$5

  metadata_tmp=$(mktemp "$cache_root/.fetch-guidance.metadata.XXXXXX")
  {
    printf 'format=1\n'
    printf 'effective_url=%s\n' "$effective_url"
    printf 'etag=%s\n' "$etag"
    printf 'last_modified=%s\n' "$last_modified"
    printf 'validated_at_epoch=%s\n' "$validated_epoch"
    printf 'validated_at_utc=%s\n' "$validated_utc"
  } >"$metadata_tmp"
  mv -f "$metadata_tmp" "$metadata_file"
  metadata_tmp=''
}

current_format=$(metadata_value format "$metadata_file" 2>/dev/null || true)
current_effective_url=$(metadata_value effective_url "$metadata_file" 2>/dev/null || true)
current_etag=$(metadata_value etag "$metadata_file" 2>/dev/null || true)
current_last_modified=$(metadata_value last_modified "$metadata_file" 2>/dev/null || true)
current_validated_epoch=$(metadata_value validated_at_epoch "$metadata_file" 2>/dev/null || true)
current_validated_utc=$(metadata_value validated_at_utc "$metadata_file" 2>/dev/null || true)

have_valid_cache=false
if [ "$current_format" = 1 ] && [ -n "$current_validated_utc" ] && \
  allowed_effective_url "$current_effective_url" && validate_body "$body_file"; then
  case "$current_validated_epoch" in
    ''|*[!0-9]*) ;;
    *) have_valid_cache=true ;;
  esac
fi

cache_age=''
if [ "$have_valid_cache" = true ]; then
  now_epoch=$(date -u +%s)
  cache_age=$((now_epoch - current_validated_epoch))
fi

if [ "$refresh" = false ] && [ "$have_valid_cache" = true ] && \
  [ "$cache_age" -ge 0 ] && [ "$cache_age" -le "$fresh_limit_seconds" ]; then
  printf 'agents-brain: cached %s last validated at %s (%s)\n' \
    "$artifact" "$current_validated_utc" "$current_effective_url" >&2
  printf '%s\n' "$body_file"
  exit 0
fi

headers_tmp=$(mktemp "$cache_root/.fetch-guidance.headers.XXXXXX")
body_tmp=$(mktemp "$cache_root/.fetch-guidance.body.XXXXXX")

response_code=''
effective_url=''

fetch_once() {
  local conditional=$1
  local curl_output
  local curl_rc
  local curl_args

  : >"$headers_tmp"
  : >"$body_tmp"

  curl_args=(
    --connect-timeout 10
    --dump-header "$headers_tmp"
    --fail
    --location
    --max-time 30
    --output "$body_tmp"
    --proto '=https'
    --proto-redir '=https'
    --show-error
    --silent
    --write-out '%{http_code}\n%{url_effective}\n'
  )

  if [ "$conditional" = true ] && [ -n "$current_etag" ]; then
    curl_args+=(--header "If-None-Match: $current_etag")
  elif [ "$conditional" = true ] && [ -n "$current_last_modified" ]; then
    curl_args+=(--header "If-Modified-Since: $current_last_modified")
  fi

  set +e
  curl_output=$(curl "${curl_args[@]}" "$source_url")
  curl_rc=$?
  set -e
  if [ "$curl_rc" -ne 0 ]; then
    return 1
  fi

  response_code=$(printf '%s\n' "$curl_output" | sed -n '1p')
  effective_url=$(printf '%s\n' "$curl_output" | sed -n '2p')
  return 0
}

print_success() {
  local cache_state=$1
  local validated_utc=$2
  local final_url=$3

  printf 'agents-brain: %s %s at %s (%s)\n' "$cache_state" "$artifact" "$validated_utc" "$final_url" >&2
  printf '%s\n' "$body_file"
}

use_stale_cache() {
  local now_epoch
  local stale_age

  [ "$refresh" = false ] || return 1
  [ "$have_valid_cache" = true ] || return 1

  now_epoch=$(date -u +%s)
  stale_age=$((now_epoch - current_validated_epoch))
  [ "$stale_age" -ge 0 ] || return 1
  [ "$stale_age" -le "$stale_limit_seconds" ] || return 1

  printf 'agents-brain: stale %s last validated at %s (%s); live retrieval failed\n' \
    "$artifact" "$current_validated_utc" "$current_effective_url" >&2
  printf '%s\n' "$body_file"
  return 0
}

conditional=false
if [ "$refresh" = false ] && [ "$have_valid_cache" = true ]; then
  if [ -n "$current_etag" ] || [ -n "$current_last_modified" ]; then
    conditional=true
  fi
fi

if ! fetch_once "$conditional"; then
  if [ "$refresh" = true ]; then
    die "forced refresh failed for $artifact"
  fi
  if use_stale_cache; then
    exit 0
  fi
  die "could not retrieve $artifact and no cache validated within seven days is available"
fi

if ! allowed_effective_url "$effective_url"; then
  die "refused unexpected final URL for $artifact: $effective_url"
fi

if [ "$response_code" = 304 ]; then
  if [ "$have_valid_cache" != true ]; then
    if ! fetch_once false; then
      die "received 304 for unusable $artifact cache and unconditional retrieval failed"
    fi
    if ! allowed_effective_url "$effective_url"; then
      die "refused unexpected final URL for $artifact: $effective_url"
    fi
  else
    validated_epoch=$(date -u +%s)
    validated_utc=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
    write_metadata "$effective_url" "$current_etag" "$current_last_modified" "$validated_epoch" "$validated_utc"
    print_success revalidated "$validated_utc" "$effective_url"
    exit 0
  fi
fi

if [ "$response_code" != 200 ]; then
  die "unexpected HTTP status for $artifact: $response_code"
fi

if ! validate_body "$body_tmp"; then
  die "retrieved invalid content for $artifact"
fi

new_etag=$(final_header_value ETag "$headers_tmp")
new_last_modified=$(final_header_value Last-Modified "$headers_tmp")
validated_epoch=$(date -u +%s)
validated_utc=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

mv -f "$body_tmp" "$body_file"
body_tmp=''
write_metadata "$effective_url" "$new_etag" "$new_last_modified" "$validated_epoch" "$validated_utc"
print_success fetched "$validated_utc" "$effective_url"
