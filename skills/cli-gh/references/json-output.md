# JSON Output

> When to read: Read when the user wants to use the `--json` flag for scriptable output, or needs to know the correct gh CLI JSON field names (which differ from GitHub API names).

Use `--json` flag for structured output. **Always verify field names with `--help`** as they differ from GitHub API names.

```bash
# Check available JSON fields for any command
gh repo view --help | grep -A 50 "JSON FIELDS"
gh pr list --help | grep -A 50 "JSON FIELDS"
```

## Common JSON Field Corrections

| Wrong (API-style) | Correct (gh CLI) |
| ----------------- | ---------------- |
| `stargazersCount` | `stargazerCount` |
| `forksCount`      | `forkCount`      |
| `watchersCount`   | `watchers`       |
| `openIssuesCount` | `issues`         |

## Repository View Fields

```bash
# Common fields for gh repo view --json
gh repo view owner/repo --json name,description,stargazerCount,forkCount,updatedAt,url,readme
```
