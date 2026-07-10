# Codespaces

> When to read: Read when the user asks to list, create, connect to, or manage files within GitHub Codespaces from the
> terminal.

Manage GitHub Codespaces directly from the terminal.

## List and Create Codespaces

```bash
# List codespaces
gh codespace list

# Create new codespace
gh codespace create --repo owner/repo
```

## Connect to Codespaces

```bash
# SSH into codespace
gh codespace ssh

# Open in VS Code
gh codespace code

# Open in JupyterLab
gh codespace jupyter
```

## Manage Codespace Files

```bash
# Copy files to/from codespace
gh codespace cp local-file.txt remote:~/path/
gh codespace cp remote:~/path/file.txt ./local-dir/

# View logs
gh codespace logs
```
