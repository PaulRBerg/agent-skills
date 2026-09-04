#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031

setup() {
  # shellcheck disable=SC2154
  repo_root=$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)
  helper=$repo_root/skills/agents-brain/scripts/fetch-guidance.sh
  test_root=$(mktemp -d "${TMPDIR:-/tmp}/agents-brain-tests.XXXXXX")
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
  *latest-model/gpt-6-astra.md)
    effective_url='https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md'
    artifact_kind=gpt
    ;;
  *prompting-claude-fable-5-1.md)
    effective_url='https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1.md'
    artifact_kind=claude
    ;;
  *)
    printf 'unexpected source URL: %s\n' "$source_url" >&2
    exit 64
    ;;
esac

write_body() {
  version=$1
  if [ "$artifact_kind" = gpt ]; then
    {
      printf '%s\n' '# Using GPT-6 Astra' "version=$version"
      index=0
      while [ "$index" -lt 80 ]; do
        printf 'GPT prompting fixture padding line %s.\n' "$index"
        index=$((index + 1))
      done
    } >"$output_file"
  else
    {
      printf '%s\n' '---' 'title: Prompting Claude Fable 5.1' '---' "version=$version"
      index=0
      while [ "$index" -lt 80 ]; do
        printf 'Claude prompting fixture padding line %s.\n' "$index"
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
  response_code=200
  write_headers "$response_code" "$etag" "$last_modified"
  write_body "$version"
}

write_not_modified() {
  response_code=304
  write_headers "$response_code" '' ''
  : >"$output_file"
}

write_normal() {
  if [ "$artifact_kind" = gpt ]; then
    if [ "$conditional_header" = 'If-None-Match: "v1"' ]; then
      write_not_modified
    else
      write_ok v1 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    fi
  else
    write_ok v1 '' ''
  fi
}

mode=${FAKE_CURL_MODE:-normal}
case "$mode" in
  failure)
    printf 'curl: simulated retrieval failure\n' >&2
    exit 7
    ;;
  bad-content)
    response_code=200
    write_headers "$response_code" '"bad"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    printf 'not prompting guidance\n' >"$output_file"
    ;;
  bad-url)
    effective_url='https://example.com/prompting.md'
    write_ok v1 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    ;;
  last-modified)
    if [ "$conditional_header" = 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' ]; then
      write_not_modified
    else
      write_ok v1 '' 'Tue, 11 Aug 2026 08:00:00 GMT'
    fi
    ;;
  recover-304)
    if [ "$count" -eq 1 ]; then
      write_not_modified
    else
      write_ok v1 '"v1"' 'Tue, 11 Aug 2026 08:00:00 GMT'
    fi
    ;;
  slow)
    sleep 1
    write_normal
    ;;
  updated)
    if [ "$artifact_kind" = gpt ]; then
      write_ok v2 '"v2"' 'Tue, 11 Aug 2026 09:00:00 GMT'
    else
      write_ok v2 '' ''
    fi
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

  export AGENTS_BRAIN_CACHE_DIR=$cache_dir
  export FAKE_CURL_STATE_DIR=$fake_state
  export FAKE_CURL_MODE=normal
  export PATH=$fake_bin:$PATH
}

teardown() {
  rm -rf "$test_root"
}

age_cache() {
  artifact=$1
  age_seconds=$2
  now_epoch=$(date -u +%s)
  aged_epoch=$((now_epoch - age_seconds))
  sed "s/^validated_at_epoch=.*/validated_at_epoch=$aged_epoch/" "$cache_dir/$artifact.meta" >"$test_root/meta.next"
  mv "$test_root/meta.next" "$cache_dir/$artifact.meta"
}

@test 'fetches and reuses a fresh GPT-6 Astra guide without revalidation' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched gpt-6-astra'* ]]
  [ -s "$cache_dir/gpt-6-astra-prompting.md" ]
  validated_epoch=$(sed -n 's/^validated_at_epoch=//p' "$cache_dir/gpt-6-astra.meta")

  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'cached gpt-6-astra last validated at'* ]]
  [ "$(sed -n 's/^validated_at_epoch=//p' "$cache_dir/gpt-6-astra.meta")" = "$validated_epoch" ]
  [ "$(cat "$fake_state/count")" -eq 1 ]
  grep -Fq 'version=v1' "$cache_dir/gpt-6-astra-prompting.md"
}

@test 'conditionally revalidates an older GPT-6 Astra guide with its ETag' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  age_cache gpt-6-astra 86401

  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'revalidated gpt-6-astra'* ]]
  grep -Fq 'If-None-Match: "v1"' "$fake_state/requests"
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'conditionally revalidates with Last-Modified when no ETag is available' {
  export FAKE_CURL_MODE=last-modified
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  age_cache gpt-6-astra 86401

  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'revalidated gpt-6-astra'* ]]
  grep -Fq 'If-Modified-Since: Tue, 11 Aug 2026 08:00:00 GMT' "$fake_state/requests"
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'supports the fixed Claude guide and refreshes it without validators' {
  run "$helper" claude-fable-5-1
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched claude-fable-5-1'* ]]
  grep -Fq 'title: Prompting Claude Fable 5.1' "$cache_dir/claude-fable-5-1-prompting.md"

  run "$helper" claude-fable-5-1
  [ "$status" -eq 0 ]
  [[ "$output" == *'cached claude-fable-5-1 last validated at'* ]]
  age_cache claude-fable-5-1 86401

  run "$helper" claude-fable-5-1
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched claude-fable-5-1'* ]]
  [ "$(cat "$fake_state/count")" -eq 2 ]
  run grep -Fq 'If-' "$fake_state/requests"
  [ "$status" -eq 1 ]
}

@test 'atomically replaces changed content and metadata' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]

  age_cache gpt-6-astra 86401
  export FAKE_CURL_MODE=updated
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched gpt-6-astra'* ]]
  grep -Fq 'version=v2' "$cache_dir/gpt-6-astra-prompting.md"
  grep -Fq 'etag="v2"' "$cache_dir/gpt-6-astra.meta"
}

@test 'uses a recently validated cache after retrieval failure' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  age_cache gpt-6-astra 86401

  export FAKE_CURL_MODE=failure
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'stale gpt-6-astra'* ]]
  [[ "$output" == *"$cache_dir/gpt-6-astra-prompting.md"* ]]
}

@test 'rejects expired fallback entries and forced-refresh failures' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  age_cache gpt-6-astra 604900

  export FAKE_CURL_MODE=failure
  run "$helper" gpt-6-astra
  [ "$status" -ne 0 ]
  [[ "$output" == *'no cache validated within seven days'* ]]

  export FAKE_CURL_MODE=normal
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  export FAKE_CURL_MODE=failure
  run "$helper" --refresh gpt-6-astra
  [ "$status" -ne 0 ]
  [[ "$output" == *'forced refresh failed for gpt-6-astra'* ]]
}

@test 'fails closed on invalid content or an unexpected redirect' {
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  original_checksum=$(cksum "$cache_dir/gpt-6-astra-prompting.md")

  export FAKE_CURL_MODE=bad-content
  run "$helper" --refresh gpt-6-astra
  [ "$status" -ne 0 ]
  [[ "$output" == *'retrieved invalid content'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/gpt-6-astra-prompting.md")" ]

  export FAKE_CURL_MODE=bad-url
  run "$helper" --refresh gpt-6-astra
  [ "$status" -ne 0 ]
  [[ "$output" == *'refused unexpected final URL'* ]]
  [ "$original_checksum" = "$(cksum "$cache_dir/gpt-6-astra-prompting.md")" ]
}

@test 'recovers from an unconditional 304 with one full retry' {
  export FAKE_CURL_MODE=recover-304
  run "$helper" gpt-6-astra
  [ "$status" -eq 0 ]
  [[ "$output" == *'fetched gpt-6-astra'* ]]
  [ "$(cat "$fake_state/count")" -eq 2 ]
}

@test 'serializes concurrent writers' {
  export FAKE_CURL_MODE=slow
  "$helper" gpt-6-astra >"$test_root/first.out" 2>"$test_root/first.err" &
  first_pid=$!
  sleep 0.1
  "$helper" gpt-6-astra >"$test_root/second.out" 2>"$test_root/second.err" &
  second_pid=$!

  wait "$first_pid"
  first_rc=$?
  wait "$second_pid"
  second_rc=$?

  [ "$first_rc" -eq 0 ]
  [ "$second_rc" -eq 0 ]
  [ "$(cat "$fake_state/count")" -eq 1 ]
  grep -Fq 'version=v1' "$cache_dir/gpt-6-astra-prompting.md"
  grep -Fq 'cached gpt-6-astra' "$test_root/second.err"
}

@test 'rejects unknown artifacts' {
  run "$helper" unknown
  [ "$status" -eq 64 ]
  [[ "$output" == *"unknown artifact 'unknown'"* ]]
}
