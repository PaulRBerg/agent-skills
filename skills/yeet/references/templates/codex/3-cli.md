# CLI Bug Template

Use these headings exactly for `openai/codex` issues using `3-cli.yml`.

```markdown
### What version of Codex CLI is running?

{output of `codex --version`}

### What subscription do you have?

{Plus/Pro/Team/Enterprise/Free}

### Which model were you using?

{e.g. gpt-5.2, gpt-5.2-codex — or omit if not specified}

### What platform is your computer?

{see references/commons.md > Platform String Normalization}

### What terminal emulator and version are you using (if applicable)?

{e.g. iTerm2 3.5.0, Ghostty 1.0, Terminal.app, VS Code integrated terminal, Cursor integrated terminal}
{note any multiplexer: tmux / screen / zellij}

### Codex doctor report

{output of `codex doctor --json`, or `not available`; review for secrets before posting}

### What issue are you seeing?

{describe the bug; include thread id if applicable}

### What steps can reproduce the bug?

1. {step}
2. {step}

### What is the expected behavior?

{expected behavior}

### Additional information

{anything else, or omit}
```
