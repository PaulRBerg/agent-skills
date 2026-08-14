#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031

setup() {
  # shellcheck disable=SC2154
  repo_root=$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)
  helper=$repo_root/skills/skill-writing/scripts/fetch-agentskills-spec.sh
  test_root=$(mktemp -d "${TMPDIR:-/tmp}/agentskills-cache-tests.XXXXXX")
  cache_dir=$test_root/cache
  fake_bin=$test_root/bin
  fake_state=$test_root/state
  mkdir -p "$cache_dir" "$fake_bin" "$fake_state"
  cache_dir=$(cd "$cache_dir" && pwd -P)

  cat >"$fake_bin/curl" <<'EOF'
#!/bin/bash
set -euo pipefail

headers_file=''
output_file=''
conditional_header=''
source_url=''

while [ "$#" -gt 0 ]; do
  case "$1" in
    --connect-timeout|--dump-header|--header|--max-time|--output|--proto|--proto-redir|--write-out)
      option=$1
      value=$2
      case "$option" in
        --dump-header) headers_file=$value ;;
        --header) conditional_header=$value ;;
        --output) output_file=$value ;;
      esac
      shift 2
      ;;
    --fail|--location|--show-error|--silent)
      shift
      ;;
    *)
      source_url=$1
      shift
      ;;
  esac
done

: "${FAKE_CURL_STATE_DIR:?}"
: "${headers_file:?}"
: "${output_file:?}"
[ "$source_url" = 'https://agentskills.io/specification.md' ] || {
  printf 'unexpected source URL: %s\n' "$source_url" >&2
  exit 64
}

count_file=$FAKE_CURL_STATE_DIR/count
if [ -f "$count_file" ]; then
  count=$(cat "$count_file")
else
  count=0
fi
count=$((count + 1))
printf '%s\n' "$count" >"$count_file"
printf '%s\n' "$conditional_header" >>"$FAKE_CURL_STATE_DIR/requests"

effective_url='https://agentskills.io/specification.md'
response_code=''

write_body() {
  version=$1
  {
    printf '%s\n' '# Specification' "version=$version" '## `SKILL.md` format'
    index=0
    while [ "$index" -lt 80 ]; do
      printf 'Agent Skills specification fixture padding line %s.\n' "$index"
      index=$((index + 1))
    done
  } >"$output_file"
}

write_headers() {
  response_code=$1
  etag=$2
  last_modified=$3
  {
    printf 'HTTP/2 %s\r\n' "$response_code"
    if [ -n "$etag" ]; then
      printf 'ETag: %s\r\n' "$etag"
    fi
    if [ -n "$last_modified" ]; then
      printf 'Last-Modified: %s\r\n' "$last_modified"
    fi
    printf '\r\n'
  } >"$headers_file"
}

write_ok() {
  version=$1
  etag=$2
  last_modified=$3
  write_headers 200 "$etag" "$last_modified"
  write_body "$version"
}

write_not_modified() {
  write_headers 304 '' ''
  : >"$output_file"
}

write_normal() {
  if [ "$conditional_header" = 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' ]; then
    write_not_modified
  else
    write_ok v1 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
  fi
}

mode=${FAKE_CURL_MODE:-normal}
case "$mode" in
  failure)
    printf 'curl: simulated retrieval failure\n' >&2
    exit 7
    ;;
  bad-content)
    write_headers 200 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
    printf 'not the specification\n' >"$output_file"
    ;;
  bad-url)
    effective_url='https://example.com/specification.md'
    write_ok v1 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
    ;;
  recover-304)
    if [ "$count" -eq 1 ]; then
      write_not_modified
    else
      write_ok v1 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
    fi
    ;;
  slow)
    sleep 1
    write_normal
    ;;
  updated)
    write_ok v2 '"v2"' 'Tue, 11 Aug 2026 09:00:00 GMT'
    ;;
  normal)
    write_normal
    ;;
  *)
    printf 'unknown fake curl mode: %s\n' "$mode" >&2
    exit 64
    ;;
esac

printf '%s\n%s\n' "$response_code" "$effective_url"
EOF
  chmod 755 "$fake_bin/curl"

  export AGENTSKILLS_CACHE_DIR=$cache_dir
  export FAKE_CURL_STATE_DIR=$fake_state
  export FAKE_CURL_MODE=normal
  export PATH=$fake_bin:$PATH
}

teardown() {
  rm -rf "$test_root"
}

age_cache() {
  age_seconds=$1
  now_epoch=$(date -u +%s)
  aged_epoch=$((now_epoch - age_seconds))
  sed "s/^validated_at_epoch=.*/validated_at_epoch=$aged_epoch/" "$cache_dir/specification.meta" >"$test_root/meta.next"
  mv "$test_root/meta.next" "$cache_dir/specification.meta"
}

@test 'fetches and reuses a fresh specification without network access' {
  run "$helper"
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched specification'* ]]
  [ -s "$cache_dir/specification.md" ]
  validated_epoch=$(sed -n 's/^validated_at_epoch=//p' "$cache_dir/specification.meta")

  run "$helper"
  [ "$status" -eq 0 ]
  [[ "$output" == *'cached specification last validated at'* ]]
  [ "$(sed -n 's/^validated_at_epoch=//p' "$cache_dir/specification.meta")" = "$validated_epoch" ]
  [ "$(cat "$fake_state/count")" -eq 1 ]
  grep -Fq 'version=v1' "$cache_dir/specification.md"
}

@test 'conditionally revalidates an older specification' {
  run "$helper"
  [ "$status" -eq 0 ]
  age_cache 86401

  run "$helper"
  [ "$status" -eq 0 ]
  [[ "$output" == *'revalidated specification'* ]]
  grep -Fq 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' "$fake_state/requests"
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'forced refresh atomically replaces changed content and metadata' {
  run "$helper"
  [ "$status" -eq 0 ]

  export FAKE_CURL_MODE=updated
  run "$helper" --refresh
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched specification'* ]]
  grep -Fq 'version=v2' "$cache_dir/specification.md"
  grep -Fq 'etag="v2"' "$cache_dir/specification.meta"
}

@test 'uses a recently validated cache after retrieval failure' {
  run "$helper"
  [ "$status" -eq 0 ]
  age_cache 86401

  export FAKE_CURL_MODE=failure
  run "$helper"
  [ "$status" -eq 0 ]
  [[ "$output" == *'stale specification'* ]]
  [[ "$output" == *"$cache_dir/specification.md"* ]]
}

@test 'rejects expired fallback entries and forced-refresh failures' {
  run "$helper"
  [ "$status" -eq 0 ]
  age_cache 604900

  export FAKE_CURL_MODE=failure
  run "$helper"
  [ "$status" -ne 0 ]
  [[ "$output" == *'no cache validated within seven days'* ]]

  export FAKE_CURL_MODE=normal
  run "$helper"
  [ "$status" -eq 0 ]
  export FAKE_CURL_MODE=failure
  run "$helper" --refresh
  [ "$status" -ne 0 ]
  [[ "$output" == *'forced refresh failed for specification'* ]]
}

@test 'fails closed without replacing the cache on invalid content or URL' {
  run "$helper"
  [ "$status" -eq 0 ]
  original_checksum=$(cksum "$cache_dir/specification.md")

  export FAKE_CURL_MODE=bad-content
  run "$helper" --refresh
  [ "$status" -ne 0 ]
  [[ "$output" == *'retrieved invalid content'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/specification.md")" ]

  export FAKE_CURL_MODE=bad-url
  run "$helper" --refresh
  [ "$status" -ne 0 ]
  [[ "$output" == *'refused unexpected final URL'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/specification.md")" ]
}

@test 'recovers from an unconditional 304 with one full retry' {
  export FAKE_CURL_MODE=recover-304
  run "$helper"
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched specification'* ]]
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'serializes concurrent writers' {
  export FAKE_CURL_MODE=slow
  "$helper" >"$test_root/first.out" 2>"$test_root/first.err" &
  first_pid=$!
  sleep 0.1
  "$helper" >"$test_root/second.out" 2>"$test_root/second.err" &
  second_pid=$!

  wait "$first_pid"
  first_rc=$?
  wait "$second_pid"
  second_rc=$?

  [ "$first_rc" -eq 0 ]
  [ "$second_rc" -eq 0 ]
  [ "$(cat "$fake_state/count")" -eq 1 ]
  grep -Fq 'version=v1' "$cache_dir/specification.md"
  grep -Fq 'cached specification' "$test_root/second.err"
}

@test 'rejects unexpected arguments' {
  run "$helper" unexpected
  [ "$status" -eq 64 ]
  [[ "$output" == *'Usage: fetch-agentskills-spec.sh [--refresh]'* ]]
}
