# Claude Code Model Behavior Template

Use these headings and dropdown values exactly for `anthropics/claude-code` issues using `model_behavior.yml`.

````markdown
### Preflight Checklist

- [x] I have searched existing issues for similar behavior reports
- [x] This report does NOT contain sensitive information (API keys, passwords, etc.)

### Type of Behavior Issue

{one of: Claude modified files I didn't ask it to modify | Claude accessed files outside the working directory | Claude ignored my instructions or configuration | Claude reverted/undid previous changes without asking | Claude made incorrect assumptions about my project | Claude refused a reasonable request | Claude's behavior changed between sessions | Subagent behaved unexpectedly | Other unexpected behavior}

### What You Asked Claude to Do

{the exact prompt or command}

### What Claude Actually Did

{step-by-step what happened}

### Expected Behavior

{what should Claude have done}

### Files Affected

{paste in a fenced ```shell block: files modified, files read unexpectedly — omit section if not applicable}

### Permission Mode

{one of: Accept Edits was ON (auto-accepting changes) | Accept Edits was OFF (manual approval required) | I don't know / Not sure}

### Can You Reproduce This?

{one of: Yes, every time with the same prompt | Sometimes (intermittent) | No, only happened once | Haven't tried to reproduce}

### Steps to Reproduce

{minimal steps — omit section if not reproducible}

### Claude Model

{one of: Sonnet | Opus | Haiku | Not sure | Other}

### Relevant Conversation

{paste relevant Claude responses in a fenced ```markdown block — omit section if none}

### Impact

{one of: Critical - Data loss or corrupted project | High - Significant unwanted changes | Medium - Extra work to undo changes | Low - Minor inconvenience}

### Claude Code Version

{output of `claude --version`, e.g., "1.0.123 (Claude Code)"}

### Platform

{one of: Anthropic API | AWS Bedrock | Google Vertex AI | Other}

### Additional Context

{patterns noticed, similar behavior, file types that trigger it — omit section if none}
````
