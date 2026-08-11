#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031

setup() {
  # shellcheck disable=SC2154
  repo_root=$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)
  helper=$repo_root/skills/agents-docs/scripts/fetch-doc.sh
  test_root=$(mktemp -d "${TMPDIR:-/tmp}/agents-docs-tests.XXXXXX")
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
: "${source_url:?}"

count_file=$FAKE_CURL_STATE_DIR/count
if [ -f "$count_file" ]; then
  count=$(cat "$count_file")
else
  count=0
fi
count=$((count + 1))
printf '%s\n' "$count" >"$count_file"
printf '%s\n' "$conditional_header" >>"$FAKE_CURL_STATE_DIR/requests"

case "$source_url" in
  *config-schema.json)
    effective_url='https://learn.chatgpt.com/docs/config-schema.json'
    artifact_kind=schema
    ;;
  *)
    effective_url='https://learn.chatgpt.com/docs/codex-manual.md'
    artifact_kind=manual
    ;;
esac

write_body() {
  version=$1
  if [ "$artifact_kind" = schema ]; then
    printf '{\n  "$schema": "http://json-schema.org/draft-07/schema#",\n  "version": "%s",\n  "padding": "' "$version" >"$output_file"
    index=0
    while [ "$index" -lt 1100 ]; do
      printf 'x' >>"$output_file"
      index=$((index + 1))
    done
    printf '"\n}\n' >>"$output_file"
  else
    {
      printf '%s\n' '---' 'title: "Codex Manual"' '---' "version=$version"
      index=0
      while [ "$index" -lt 80 ]; do
        printf 'Codex documentation fixture padding line %s.\n' "$index"
        index=$((index + 1))
      done
    } >"$output_file"
  fi
}

write_headers() {
  response_code=$1
  etag=$2
  last_modified=$3
  {
    printf 'HTTP/2 308\r\nlocation: %s\r\n\r\n' "$effective_url"
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

mode=${FAKE_CURL_MODE:-normal}
case "$mode" in
  failure)
    printf 'curl: simulated retrieval failure\n' >&2
    exit 7
    ;;
  bad-content)
    write_headers 200 '"bad"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    printf 'not documentation\n' >"$output_file"
    ;;
  bad-url)
    effective_url='https://example.com/docs/codex-manual.md'
    write_headers 200 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    write_body v1
    ;;
  last-modified)
    if [ "$conditional_header" = 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' ]; then
      write_headers 304 '' ''
      : >"$output_file"
    else
      write_headers 200 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
      write_body v1
    fi
    ;;
  recover-304)
    if [ "$count" -eq 1 ]; then
      write_headers 304 '' ''
      : >"$output_file"
    else
      write_headers 200 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
      write_body v1
    fi
    ;;
  slow)
    sleep 1
    if [ "$conditional_header" = 'If-None-Match: "v1"' ]; then
      write_headers 304 '' ''
      : >"$output_file"
    else
      write_headers 200 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
      write_body v1
    fi
    ;;
  updated)
    write_headers 200 '"v2"' 'Tue, 11 Aug 2026 09:00:00 GMT'
    write_body v2
    ;;
  normal)
    if [ "$conditional_header" = 'If-None-Match: "v1"' ]; then
      write_headers 304 '' ''
      : >"$output_file"
    else
      write_headers 200 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
      write_body v1
    fi
    ;;
  *)
    printf 'unknown fake curl mode: %s\n' "$mode" >&2
    exit 64
    ;;
esac

case "$mode" in
  recover-304)
    if [ "$count" -eq 1 ]; then response_code=304; else response_code=200; fi
    ;;
  last-modified|normal|slow)
    if [ -n "$conditional_header" ]; then response_code=304; else response_code=200; fi
    ;;
  *) response_code=200 ;;
esac

printf '%s\n%s\n' "$response_code" "$effective_url"
EOF
  chmod 755 "$fake_bin/curl"

  export AGENTS_DOCS_CACHE_DIR=$cache_dir
  export FAKE_CURL_STATE_DIR=$fake_state
  export FAKE_CURL_MODE=normal
  export PATH=$fake_bin:$PATH
}

teardown() {
  rm -rf "$test_root"
}

@test 'fetches and conditionally revalidates the Codex manual' {
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched codex-manual'* ]]
  [ -s "$cache_dir/codex-manual.md" ]

  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  [[ "$output" == *'revalidated codex-manual'* ]]
  grep -Fq 'If-None-Match: "v1"' "$fake_state/requests"
  grep -Fq 'version=v1' "$cache_dir/codex-manual.md"
}

@test 'uses Last-Modified when no ETag is available' {
  export FAKE_CURL_MODE=last-modified
  run "$helper" codex-manual
  [ "$status" -eq 0 ]

  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  [[ "$output" == *'revalidated codex-manual'* ]]
  grep -Fq 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' "$fake_state/requests"
}

@test 'atomically replaces changed content and metadata' {
  run "$helper" codex-manual
  [ "$status" -eq 0 ]

  export FAKE_CURL_MODE=updated
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  grep -Fq 'version=v2' "$cache_dir/codex-manual.md"
  grep -Fq 'etag="v2"' "$cache_dir/codex-manual.meta"
}

@test 'supports the fixed configuration schema artifact' {
  run "$helper" codex-config-schema
  [ "$status" -eq 0 ]
  [[ "$output" == *"$cache_dir/config-schema.json"* ]]
  grep -Fq "\"\$schema\":" "$cache_dir/config-schema.json"
}

@test 'uses a recently validated cache after retrieval failure' {
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  now_epoch=$(date -u +%s)
  recent_epoch=$((now_epoch - 604700))
  sed "s/^validated_at_epoch=.*/validated_at_epoch=$recent_epoch/" "$cache_dir/codex-manual.meta" >"$cache_dir/meta.next"
  mv "$cache_dir/meta.next" "$cache_dir/codex-manual.meta"

  export FAKE_CURL_MODE=failure
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  [[ "$output" == *'stale codex-manual'* ]]
  [[ "$output" == *"$cache_dir/codex-manual.md"* ]]
}

@test 'rejects expired fallback entries and forced-refresh failures' {
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  now_epoch=$(date -u +%s)
  expired_epoch=$((now_epoch - 604900))
  sed "s/^validated_at_epoch=.*/validated_at_epoch=$expired_epoch/" "$cache_dir/codex-manual.meta" >"$cache_dir/meta.next"
  mv "$cache_dir/meta.next" "$cache_dir/codex-manual.meta"

  export FAKE_CURL_MODE=failure
  run "$helper" codex-manual
  [ "$status" -ne 0 ]
  [[ "$output" == *'no cache validated within seven days'* ]]

  export FAKE_CURL_MODE=normal
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  export FAKE_CURL_MODE=failure
  run "$helper" --refresh codex-manual
  [ "$status" -ne 0 ]
  [[ "$output" == *'forced refresh failed for codex-manual'* ]]
}

@test 'fails closed on invalid content or an unexpected redirect' {
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  original_checksum=$(cksum "$cache_dir/codex-manual.md")

  export FAKE_CURL_MODE=bad-content
  run "$helper" --refresh codex-manual
  [ "$status" -ne 0 ]
  [[ "$output" == *'retrieved invalid content'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/codex-manual.md")" ]

  export FAKE_CURL_MODE=bad-url
  run "$helper" --refresh codex-manual
  [ "$status" -ne 0 ]
  [[ "$output" == *'refused unexpected final URL'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/codex-manual.md")" ]
}

@test 'recovers from an unconditional 304 with one full retry' {
  export FAKE_CURL_MODE=recover-304
  run "$helper" codex-manual
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched codex-manual'* ]]
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'serializes concurrent writers' {
  export FAKE_CURL_MODE=slow
  "$helper" codex-manual >"$test_root/first.out" 2>"$test_root/first.err" &
  first_pid=$!
  sleep 0.1
  "$helper" codex-manual >"$test_root/second.out" 2>"$test_root/second.err" &
  second_pid=$!

  wait "$first_pid"
  first_rc=$?
  wait "$second_pid"
  second_rc=$?

  [ "$first_rc" -eq 0 ]
  [ "$second_rc" -eq 0 ]
  [ "$(cat "$fake_state/count")" -eq 2 ]
  grep -Fq 'version=v1' "$cache_dir/codex-manual.md"
  grep -Fq 'revalidated codex-manual' "$test_root/second.err"
}

@test 'rejects unknown artifacts' {
  run "$helper" unknown
  [ "$status" -eq 64 ]
  [[ "$output" == *"unknown artifact 'unknown'"* ]]
}
