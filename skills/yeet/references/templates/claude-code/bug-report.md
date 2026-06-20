# Claude Code Bug Report Template

Use these headings and dropdown values exactly for `anthropics/claude-code` issues using `bug_report.yml`.

````markdown
### Preflight Checklist

- [x] I have searched existing issues and this hasn't been reported yet
- [x] This is a single bug report (please file separate reports for different bugs)
- [x] I am using the latest version of Claude Code

### What's Wrong?

{what's happening that shouldn't be}

### What Should Happen?

{expected behavior}

### Error Messages/Logs

{paste error output, stack traces, or logs in a fenced ```shell block — omit section if none}

### Steps to Reproduce

1. {step 1}
2. {step 2}
3. ...

### Claude Model

{one of: Sonnet (default) | Opus | Not sure / Multiple models | Other}

### Is this a regression?

{one of: Yes, this worked in a previous version | No, this never worked | I don't know}

### Last Working Version

{e.g., 1.0.100 — only if regression, otherwise write "N/A"}

### Claude Code Version

{output of `claude --version`, e.g., "1.0.123 (Claude Code)"}

### Platform

{one of: Anthropic API | AWS Bedrock | Google Vertex AI | Other}

### Operating System

{one of: macOS | Windows | Ubuntu/Debian Linux | Other Linux | Other}

### Terminal/Shell

{one of: Terminal.app (macOS) | Warp | Cursor | iTerm2 | IntelliJ IDEA terminal | VS Code integrated terminal | PyCharm terminal | Windows Terminal | PowerShell | WSL (Windows Subsystem for Linux) | Xterm | Non-interactive/CI environment | Other}

### Additional Information

{screenshots, config files, repro repo links, exact OS version (e.g., "macOS Tahoe v26.2") — omit section if none}
````
