# Troubleshooting

> Reproduce with the narrowest repository command before changing configuration.

- For a hang, use `--detect-async-leaks` diagnostically: it is slower and identifies resources created by a test file
  that remain open. Before raising a timeout, investigate unawaited work, unresolved mocks, unbounded retries, unclosed
  resources, and fake-timer schedulers.
- For state that passes alone but fails in a suite, use order or `--no-file-parallelism` only to diagnose. Restore state
  where it is created and make stores, clients, caches, and stateful mocks fresh per test. Serialization is not a fix.
- For missing mocks, resolution errors, or hoisting failures, check the identity and factory rules in
  [mocking.md](mocking.md) against the configured aliases.
- For discovery, check configured include/exclude, config root, selected project, and project ownership before renaming
  a test. Clear the Vite/Vitest cache only after evidence of stale dependency, transform, alias, or config resolution.
- Preserve the repository reporter. Reporter availability and output shape vary by Vitest version.

See [async-leak detection](https://vitest.dev/config/detectasyncleaks), [CLI](https://vitest.dev/guide/cli), and
[configuration](https://vitest.dev/config/file).
