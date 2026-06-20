#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage:
  yeet-context.sh repo [owner/repo] [--issue-templates] [--discussion-templates] [--discussion-categories]
  yeet-context.sh issue [owner/repo] <number>
  yeet-context.sh labels [owner/repo]

Read-only GitHub context helper. When owner/repo is omitted, the repository is
inferred from the local origin remote.
EOF
}

REMAINING_ARGS=()

die() {
  printf 'yeet-context: %s\n' "$*" >&2
  exit 64
}

repo_from_origin() {
  local url repo

  url="$(git config --get remote.origin.url 2>/dev/null || true)"
  [ -n "$url" ] || return 1

  case "$url" in
    git@github.com:*)
      repo="${url#git@github.com:}"
      ;;
    https://github.com/*)
      repo="${url#https://github.com/}"
      ;;
    ssh://git@github.com/*)
      repo="${url#ssh://git@github.com/}"
      ;;
    *)
      return 1
      ;;
  esac

  repo="${repo%.git}"
  repo="${repo%/}"
  printf '%s\n' "$repo"
}

split_repo() {
  local repo="$1"

  case "$repo" in
    */*) ;;
    *) die "expected owner/repo, got '$repo'" ;;
  esac

  OWNER="${repo%%/*}"
  NAME="${repo#*/}"

  case "$OWNER" in
    ''|*[!A-Za-z0-9_.-]*) die "invalid owner in '$repo'" ;;
  esac

  case "$NAME" in
    ''|*/*|*[!A-Za-z0-9_.-]*) die "invalid repo name in '$repo'" ;;
  esac

  REPO="$OWNER/$NAME"
}

repo_arg_or_origin() {
  if [ "$#" -gt 0 ] && [ "${1#--}" = "$1" ]; then
    split_repo "$1"
    shift
  else
    local inferred
    inferred="$(repo_from_origin || true)"
    [ -n "$inferred" ] || die "pass owner/repo or run from a GitHub repository with origin set"
    split_repo "$inferred"
  fi

  REMAINING_ARGS=("$@")
}

repo_context() {
  local with_issue=false
  local with_discussion_templates=false
  local with_discussion_categories=false
  local query

  repo_arg_or_origin "$@"
  set -- "${REMAINING_ARGS[@]}"

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --issue-templates)
        with_issue=true
        ;;
      --discussion-templates)
        with_discussion_templates=true
        ;;
      --discussion-categories)
        with_discussion_categories=true
        ;;
      --all)
        with_issue=true
        with_discussion_templates=true
        with_discussion_categories=true
        ;;
      *)
        die "unknown repo option '$1'"
        ;;
    esac
    shift
  done

  # GraphQL variables must remain literal here.
  # shellcheck disable=SC2016
  query='
    query(
      $owner: String!
      $name: String!
      $issueTemplateExpr: String!
      $discussionTemplateExpr: String!
      $withIssueTemplates: Boolean!
      $withDiscussionTemplates: Boolean!
      $withDiscussionCategories: Boolean!
    ) {
      viewer { login }
      repository(owner: $owner, name: $name) {
        id
        nameWithOwner
        viewerPermission
        defaultBranchRef { name }
        issueTemplateTree: object(expression: $issueTemplateExpr) @include(if: $withIssueTemplates) {
          ... on Tree {
            entries { name oid type }
          }
        }
        discussionTemplateTree: object(expression: $discussionTemplateExpr) @include(if: $withDiscussionTemplates) {
          ... on Tree {
            entries { name oid type }
          }
        }
        discussionCategories(first: 25) @include(if: $withDiscussionCategories) {
          nodes { id name slug description isAnswerable }
        }
      }
    }'

  gh api graphql \
    -f query="$query" \
    -F owner="$OWNER" \
    -F name="$NAME" \
    -f issueTemplateExpr='HEAD:.github/ISSUE_TEMPLATE' \
    -f discussionTemplateExpr='HEAD:.github/DISCUSSION_TEMPLATE' \
    -F withIssueTemplates="$with_issue" \
    -F withDiscussionTemplates="$with_discussion_templates" \
    -F withDiscussionCategories="$with_discussion_categories" \
    --jq 'if .data.repository == null then error("repository not found") else .data end'
}

issue_context() {
  local inferred number query

  case "$#" in
    1)
      inferred="$(repo_from_origin || true)"
      [ -n "$inferred" ] || die "pass owner/repo or run from a GitHub repository with origin set"
      split_repo "$inferred"
      number="$1"
      ;;
    2)
      split_repo "$1"
      number="$2"
      ;;
    *)
      usage
      exit 64
      ;;
  esac

  case "$number" in
    ''|*[!0-9]*) die "expected numeric issue or PR number, got '$number'" ;;
  esac

  # GraphQL variables must remain literal here.
  # shellcheck disable=SC2016
  query='
    query($owner: String!, $name: String!, $number: Int!) {
      viewer { login }
      repository(owner: $owner, name: $name) {
        id
        nameWithOwner
        viewerPermission
        issueOrPullRequest(number: $number) {
          __typename
          ... on Issue {
            number
            title
            body
            state
            url
            author { login }
            labels(first: 50) { nodes { name } }
            assignees(first: 20) { nodes { login } }
            milestone { title }
            comments(last: 5) {
              nodes { author { login } body createdAt url }
            }
          }
          ... on PullRequest {
            number
            title
            body
            state
            url
            isDraft
            author { login }
            labels(first: 50) { nodes { name } }
            comments(last: 5) {
              nodes { author { login } body createdAt url }
            }
          }
        }
      }
    }'

  gh api graphql \
    -f query="$query" \
    -F owner="$OWNER" \
    -F name="$NAME" \
    -F number="$number" \
    --jq 'if .data.repository == null then error("repository not found") elif .data.repository.issueOrPullRequest == null then error("issue or pull request not found") else .data end'
}

labels_context() {
  local labels

  repo_arg_or_origin "$@"
  [ "${#REMAINING_ARGS[@]}" -eq 0 ] || die "labels takes only an optional owner/repo"

  labels="$(gh label list \
    --repo "$REPO" \
    --limit 200 \
    --json name,description)"

  printf '{"repository":"%s","labels":%s}\n' "$REPO" "$labels"
}

main() {
  [ "$#" -gt 0 ] || {
    usage
    exit 64
  }

  case "$1" in
    repo)
      shift
      repo_context "$@"
      ;;
    issue)
      shift
      issue_context "$@"
      ;;
    labels)
      shift
      labels_context "$@"
      ;;
    -h|--help|help)
      usage
      ;;
    *)
      usage
      exit 64
      ;;
  esac
}

main "$@"
