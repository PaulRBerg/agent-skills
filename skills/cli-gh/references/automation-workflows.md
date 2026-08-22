# Automation Workflows

> When to read: Read when the user wants ready-made gh CLI automation patterns for reports, CI monitoring, fork sync,
> releases, or repository operations.

Common workflow patterns and automation examples using gh CLI.

Use `yeet` for pull request, issue, discussion, or comment creation and updates. This reference covers operational
automation.

## Advanced Automation Patterns

### Daily Standup Report

```bash
#!/bin/bash
# Generate daily activity report

echo "## My GitHub Activity - $(date +%Y-%m-%d)"
echo ""
echo "### PRs Created"
gh pr list --author @me --search "created:$(date +%Y-%m-%d)"
echo ""
echo "### PRs Reviewed"
gh search prs "reviewed-by:@me created:$(date +%Y-%m-%d)"
echo ""
echo "### Issues Closed"
gh issue list --author @me --state closed --search "closed:$(date +%Y-%m-%d)"
```

### Sync Fork with Upstream

```bash
#!/bin/bash
# Keep fork in sync with upstream

gh repo sync owner/fork --source upstream/repo --branch main
git fetch origin main
git merge origin/main
```

### Release Checklist Automation

```bash
#!/bin/bash
# Automated release checklist

VERSION=$1

# 1. Ensure on main branch
git checkout main && git pull

# 2. Run tests
npm test || exit 1

# 3. Create release tag
git tag -a "v$VERSION" -m "Release $VERSION"
git push origin "v$VERSION"

# 4. Create GitHub release
gh release create "v$VERSION" --generate-notes

# 5. Upload artifacts
gh release upload "v$VERSION" dist/*
```

### Monitor CI Status

```bash
#!/bin/bash
# Monitor all active PRs for CI failures

gh pr list --json number,title,statusCheckRollup --jq '.[] |
  select(.statusCheckRollup.state == "FAILURE") |
  "\(.number): \(.title)"'
```

## Notifications and Monitoring

### Watch for PR Reviews

```bash
# Monitor PR for new reviews
while true; do
  REVIEWS=$(gh pr view 123 --json reviews --jq '.reviews | length')
  echo "Reviews: $REVIEWS"
  sleep 60
done
```

### Get Notified on Workflow Completion

```bash
#!/bin/bash
# Wait for workflow and send notification

RUN_ID=$1
gh run watch $RUN_ID
STATUS=$(gh run view $RUN_ID --json conclusion --jq '.conclusion')

if [ "$STATUS" = "success" ]; then
  osascript -e 'display notification "Workflow passed!" with title "GitHub Actions"'
else
  osascript -e 'display notification "Workflow failed!" with title "GitHub Actions"'
fi
```

### PR Staleness Check

```bash
#!/bin/bash
# Find stale PRs (no activity in 30 days)

gh pr list --json number,title,updatedAt --jq '.[] |
  select((now - (.updatedAt | fromdateiso8601)) > (30*86400)) |
  "\(.number): \(.title)"'
```

## Team Reporting

### Weekly Team Report

```bash
#!/bin/bash
# Generate weekly team activity summary

TEAM="myteam"
SINCE=$(date -v-7d +%Y-%m-%d)

echo "# Team Activity Report - Week of $(date +%Y-%m-%d)"
echo ""
echo "## PRs Merged"
gh search prs "team:$TEAM is:merged merged:>=$SINCE" --limit 100
echo ""
echo "## Issues Closed"
gh search issues "team:$TEAM is:closed closed:>=$SINCE" --limit 100
```

## Repository Management

### Batch Repository Creation

```bash
#!/bin/bash
# Create multiple repositories from template

TEMPLATE="org/template-repo"
REPOS=("project-a" "project-b" "project-c")

for repo in "${REPOS[@]}"; do
  gh repo create "org/$repo" --template "$TEMPLATE" --private
done
```

### Clone All Organization Repos

```bash
#!/bin/bash
# Clone all repos from an organization

ORG="myorg"
gh repo list "$ORG" --limit 1000 --json name --jq '.[].name' | \
  xargs -I {} gh repo clone "$ORG/{}"
```

### Sync Multiple Forks

```bash
#!/bin/bash
# Sync all your forks with upstream

gh repo list @me --fork --json name,parent --jq '.[] |
  "\(.name) \(.parent.owner.login)/\(.parent.name)"' | \
  while read fork upstream; do
    gh repo sync "$fork" --source "$upstream"
  done
```
