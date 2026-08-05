#!/usr/bin/env bats

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.

bats_require_minimum_version 1.5.0

readonly SCRIPT="$BATS_TEST_DIRNAME/../../skills/yeet/scripts/yeet-context.sh"

write_mock() {
  local name="$1"
  cat > "$MOCK_BIN/$name"
  chmod +x "$MOCK_BIN/$name"
}

set_origin() {
  GIT_REMOTE="$1"
  export GIT_REMOTE
}

assert_graphql_args() {
  local expected="$1"
  [ "$(<"$GH_ARGS")" = "$expected" ]
}

assert_query_contains() {
  local expected="$1"
  [[ "$(<"$GH_QUERY")" == *"$expected"* ]]
}

setup() {
  export HOME="$BATS_TEST_TMPDIR/home"
  export MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  export GH_ARGS="$BATS_TEST_TMPDIR/gh-args"
  export GH_QUERY="$BATS_TEST_TMPDIR/gh-query"
  export PATH="$MOCK_BIN:/usr/bin:/bin"
  export GIT_REMOTE=''
  export GH_OUTPUT='{"data":{"repository":{"nameWithOwner":"acme/widgets"}}}'

  mkdir -p "$HOME" "$MOCK_BIN"
  write_mock git <<'EOF'
#!/usr/bin/env bash
set -eu
if [ "$1" = config ] && [ "$2" = --get ] && [ "$3" = remote.origin.url ]; then
  printf '%s\n' "${GIT_REMOTE:-}"
fi
EOF
  write_mock gh <<'EOF'
#!/usr/bin/env bash
set -eu
: > "$GH_ARGS"
: > "$GH_QUERY"
for argument in "$@"; do
  case "$argument" in
    query=*)
      printf '%s' "${argument#query=}" > "$GH_QUERY"
      argument=QUERY_ARG
      ;;
  esac
  printf '%s\n' "$argument" >> "$GH_ARGS"
done
printf '%s\n' "${GH_OUTPUT}"
EOF
}

@test "repo accepts an explicit repository and passes its default GraphQL arguments" {
  run "$SCRIPT" repo acme/widgets

  [ "$status" -eq 0 ]
  [ "$output" = "$GH_OUTPUT" ]
  assert_graphql_args "$(cat <<'EOF'
api
graphql
-f
QUERY_ARG
-F
owner=acme
-F
name=widgets
-f
issueTemplateExpr=HEAD:.github/ISSUE_TEMPLATE
-f
discussionTemplateExpr=HEAD:.github/DISCUSSION_TEMPLATE
-F
withIssueTemplates=false
-F
withDiscussionTemplates=false
-F
withDiscussionCategories=false
--jq
if .data.repository == null then error("repository not found") else .data end
EOF
)"
  assert_query_contains 'viewer { login }'
  assert_query_contains 'issueTemplateTree:'
}

@test "repo maps selected optional context flags to GraphQL booleans" {
  run "$SCRIPT" repo acme/widgets --issue-templates --discussion-categories

  [ "$status" -eq 0 ]
  assert_graphql_args "$(cat <<'EOF'
api
graphql
-f
QUERY_ARG
-F
owner=acme
-F
name=widgets
-f
issueTemplateExpr=HEAD:.github/ISSUE_TEMPLATE
-f
discussionTemplateExpr=HEAD:.github/DISCUSSION_TEMPLATE
-F
withIssueTemplates=true
-F
withDiscussionTemplates=false
-F
withDiscussionCategories=true
--jq
if .data.repository == null then error("repository not found") else .data end
EOF
)"
}

@test "repo --all enables every optional context request" {
  run "$SCRIPT" repo acme/widgets --all

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'withIssueTemplates=true\n-F\nwithDiscussionTemplates=true\n-F\nwithDiscussionCategories=true'* ]]
}

@test "repo infers GitHub scp origin remotes" {
  set_origin 'git@github.com:acme/widgets.git'

  run "$SCRIPT" repo

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'owner=acme\n-F\nname=widgets'* ]]
}

@test "repo infers GitHub HTTPS origin remotes" {
  set_origin 'https://github.com/acme/widgets.git'

  run "$SCRIPT" repo

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'owner=acme\n-F\nname=widgets'* ]]
}

@test "repo infers GitHub SSH URL origin remotes" {
  set_origin 'ssh://git@github.com/acme/widgets.git'

  run "$SCRIPT" repo

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'owner=acme\n-F\nname=widgets'* ]]
}

@test "repo rejects unsupported and missing origin remotes" {
  set_origin 'https://example.com/acme/widgets.git'

  run "$SCRIPT" repo

  [ "$status" -eq 64 ]
  [[ "$output" == *'pass owner/repo or run from a GitHub repository with origin set'* ]]
  [ ! -s "$GH_ARGS" ]

  set_origin ''
  run "$SCRIPT" repo

  [ "$status" -eq 64 ]
  [[ "$output" == *'pass owner/repo or run from a GitHub repository with origin set'* ]]
}

@test "repo rejects malformed repository names and unknown options" {
  run "$SCRIPT" repo acme/widgets/extra

  [ "$status" -eq 64 ]
  [[ "$output" == *"invalid repo name in 'acme/widgets/extra'"* ]]
  [ ! -s "$GH_ARGS" ]

  run "$SCRIPT" repo acme/widgets --not-a-context-option

  [ "$status" -eq 64 ]
  [[ "$output" == *"unknown repo option '--not-a-context-option'"* ]]
}

@test "issue accepts explicit repository and numeric number with exact GraphQL arguments" {
  run "$SCRIPT" issue acme/widgets 42

  [ "$status" -eq 0 ]
  [ "$output" = "$GH_OUTPUT" ]
  assert_graphql_args "$(cat <<'EOF'
api
graphql
-f
QUERY_ARG
-F
owner=acme
-F
name=widgets
-F
number=42
--jq
if .data.repository == null then error("repository not found") elif .data.repository.issueOrPullRequest == null then error("issue or pull request not found") else .data end
EOF
)"
  assert_query_contains "issueOrPullRequest(number: \$number)"
}

@test "issue infers its repository from origin when given only a number" {
  set_origin 'git@github.com:acme/widgets.git'

  run "$SCRIPT" issue 7

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'owner=acme\n-F\nname=widgets\n-F\nnumber=7'* ]]
}

@test "issue rejects non-numeric numbers and malformed repositories" {
  local number
  for number in '' '-1' '1.5' 'issue-7'; do
    run "$SCRIPT" issue acme/widgets "$number"
    [ "$status" -eq 64 ]
    [[ "$output" == *'expected numeric issue or PR number'* ]]
  done

  run "$SCRIPT" issue acme/widgets/extra 1

  [ "$status" -eq 64 ]
  [[ "$output" == *"invalid repo name in 'acme/widgets/extra'"* ]]
}

@test "issue rejects unsupported arity and unavailable inferred remotes" {
  run "$SCRIPT" issue

  [ "$status" -eq 64 ]
  [[ "$output" == *'Usage:'* ]]

  set_origin 'git@github.com:acme/widgets/extra.git'
  run "$SCRIPT" issue 9

  [ "$status" -eq 64 ]
  [[ "$output" == *"invalid repo name in 'acme/widgets/extra'"* ]]
}

@test "labels uses explicit repository, exact gh arguments, and wraps labels JSON" {
  export GH_OUTPUT='[{"name":"bug","description":"A bug"}]'

  run "$SCRIPT" labels acme/widgets

  [ "$status" -eq 0 ]
  [ "$output" = '{"repository":"acme/widgets","labels":[{"name":"bug","description":"A bug"}]}' ]
  [ "$(<"$GH_ARGS")" = "$(cat <<'EOF'
label
list
--repo
acme/widgets
--limit
200
--json
name,description
EOF
)" ]
}

@test "labels infers its repository and rejects extra or malformed input" {
  set_origin 'https://github.com/acme/widgets.git'

  run "$SCRIPT" labels

  [ "$status" -eq 0 ]
  [[ "$(<"$GH_ARGS")" == *$'--repo\nacme/widgets'* ]]

  run "$SCRIPT" labels acme/widgets extra

  [ "$status" -eq 64 ]
  [[ "$output" == *'labels takes only an optional owner/repo'* ]]

  run "$SCRIPT" labels 'acme bad/widgets'

  [ "$status" -eq 64 ]
  [[ "$output" == *"invalid owner in 'acme bad/widgets'"* ]]
}

@test "the helper reports usage for an unsupported command" {
  run "$SCRIPT" unsupported-command

  [ "$status" -eq 64 ]
  [[ "$output" == *'Usage:'* ]]
}
