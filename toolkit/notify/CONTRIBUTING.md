## Contributing

### Prerequisites

- [Python](https://www.python.org) (v3.12+)
- [uv](https://github.com/astral-sh/uv) (package manager)
- [Just](https://github.com/casey/just) (command runner)

### Setup

```bash
git clone https://github.com/PaulRBerg/ai-notify.git
cd ai-notify
uv sync
```

### Available Commands

```bash
just --list                 # Show all available commands
just install                # Install dependencies with all extras
just test                   # Run test suite
just full-check             # Run all code quality checks (ruff, pyright, prettier)
just full-write             # Auto-fix all formatting and linting issues
just ruff-check             # Run ruff linting only
just ruff-write             # Auto-fix ruff issues
just pyright-check          # Run pyright type checking
just prettier-check         # Check formatting with prettier
just prettier-write         # Auto-fix prettier formatting
```

Shorthand aliases:

```bash
just fc                     # Alias for full-check
just fw                     # Alias for full-write
```

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `just full-check` to verify code quality
5. Run `just test` to ensure all tests pass
6. Submit a pull request

### Testing

Run the test suite using pytest:

```bash
just test
```

Or with uv directly:

```bash
uv run pytest
```
