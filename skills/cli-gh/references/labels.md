# Labels

> When to read: Read when the user asks to list, create, edit, delete, or clone repository labels for issue and PR
> organization.

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

## Delete Labels

Label deletion is destructive. It removes the label from the repository and from existing issues and pull requests.

Before deleting labels:

1. Present a deletion plan with the target repository, exact label names, exact `gh label delete` commands, and the
   consequence above.
2. Wait for the user to explicitly approve that plan in a subsequent message.
3. Run only the approved commands. If anything changes, present a revised plan and wait for approval again.

```bash
# Delete one approved label
gh label delete "obsolete" --repo OWNER/REPO --yes
```
