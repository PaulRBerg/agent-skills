# Labels

> When to read: Read when the user asks to list, create, edit, or clone repository labels for issue and PR organization.

Manage repository labels for issue and PR organization.

## List and View Labels

```bash
# List all labels in repository
gh label list
```

## Create and Edit Labels

```bash
# Create new label
gh label create "priority: high" --color FF0000 --description "High priority items"

# Edit existing label
gh label edit "bug" --color FFAA00 --description "Something isn't working"
```

## Clone Labels Between Repos

```bash
# Clone labels from another repository
gh label clone owner/source-repo
```
