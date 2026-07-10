# Releases

> When to read: Read when the user asks to create, list, view, or download GitHub releases and their assets.

## Creating Releases

```bash
# Create release
gh release create v1.0.0

# Create release with notes
gh release create v1.0.0 --notes "Release notes"

# Create release with files
gh release create v1.0.0 dist/*.tar.gz

# Create draft release
gh release create v1.0.0 --draft

# Generate release notes automatically
gh release create v1.0.0 --generate-notes
```

## Managing Releases

```bash
# List releases
gh release list

# View release
gh release view v1.0.0

# Download release assets
gh release download v1.0.0

# Download from a public repo without authentication
gh release download v1.0.0 --repo owner/repo
```

`gh release download` works against public repositories without authentication (a token is still used when present),
matching `gh extension install`.
