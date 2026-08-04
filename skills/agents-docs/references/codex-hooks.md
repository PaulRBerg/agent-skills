# Codex Hooks, App-server, and Trust

Use this reference for a question about Codex hooks, managed hooks, hook trust, or automating hook configuration. Fetch
and cite the live official source first:

- [Codex hooks](https://developers.openai.com/codex/hooks)
- [Codex app-server](https://developers.openai.com/codex/app-server)

## Official behavior

The hooks page is authoritative for supported events, configuration, managed-hook behavior, and the interactive `/hooks`
surface. It documents that managed hooks are centrally controlled. Do not present an app-server operation, config key,
or on-disk trust representation as official unless the current page says so.

The app-server page is authoritative for the public JSON-RPC/app-server contract. It documents the JSONL stdio
transport, `initialize`/`initialized` handshake, `hooks/list`, `config/read`, and atomic `config/batchWrite`. Treat
exact request and response fields as version-sensitive: check the installed `codex --version` and generated schema
before relying on fields absent from the live page.

`--dangerously-bypass-hook-trust` bypasses hook trust for one Codex invocation. It is not a persistent trust grant; do
not recommend it as a replacement for a narrowly authorized configuration update.

## Local implementation notes: Codex CLI 0.146.0

Everything in this section was verified against `codex-cli 0.146.0` and its generated app-server JSON Schema. It is not
a promise about older or newer versions.

Start the app-server over stdio and exchange JSONL. Send `initialize`, wait for its response, then send the
`initialized` notification before using the v2 methods below. Generate a fresh schema in an operating-system temporary
directory when exact request or response shapes matter:

```sh
codex app-server generate-json-schema --out "$TMPDIR/codex-app-server-schema"
```

The schema exposes:

- `hooks/list`, which returns hook metadata including `key`, `source`, `sourcePath`, `isManaged`, `currentHash`, and
  `trustStatus`.
- `config/read`, which can return effective config and layers.
- `config/batchWrite`, an atomic batch edit with `filePath` and `expectedVersion`.

For a task that creates or changes hooks, use `hooks/list` to identify precisely the hooks it owns. Require an enabled,
non-managed user command hook from the active hook source path, then match its event, command, matcher, timeout, and
additional-context limit exactly against the task's authorized hook definition. Reject missing, duplicate, malformed, or
merely similar hooks; never broaden the selection to every hook in the same event, source file, project, or config
layer.

The local user-config representation for a trusted hook is:

```toml
[hooks.state."<TOML-quoted key>"]
trusted_hash = "<server-reported currentHash>"
```

For `config/batchWrite`, express the edit as `hooks.state."<TOML-double-quoted-and-escaped key>".trusted_hash`. Quote
the complete server-reported `key` as one TOML key segment, including the surrounding double quotes and TOML escapes; do
not split or normalize it. Do not calculate the hash manually. Obtain `currentHash` from the same app-server hook record
and write only the owned hook's trust state.

Read config with layers, select the user layer whose `name.file` is the active `$CODEX_HOME/config.toml`, and retain its
`version`. Submit all owned-hook trust edits in one `config/batchWrite` using that path as `filePath` and the retained
version as `expectedVersion`; this is a compare-and-swap, not a blind overwrite. If discovery, write, or verification
observes a changed version or stale state, re-run both hook and config discovery from fresh state and retry the bounded
operation. Do not replay an edit derived from stale hook metadata.

After writing, verify in a fresh Codex process: initialize, send `initialized`, list hooks again, and confirm only the
task-owned hooks have the intended trust status and current hash. Bound failures and retries. Report configuration
conflicts, malformed config, unavailable protocol methods, or a nonconverging concurrent writer instead of widening
trust or bypassing it persistently.

Authorization is narrow: an agent may trust only hooks that its authorized task created or changed. Editing a trust
entry is a configuration write; obtain the required authorization before doing it. Managed hooks remain governed by
their managed configuration and should not be treated as locally trustable task output.
