# Codex Hooks, App-server, and Trust

Use this reference for a question about Codex hooks, managed hooks, hook trust, or automating hook configuration. Fetch
and cite the live official source first:

- [Codex hooks](https://developers.openai.com/codex/hooks)
- [Codex app-server](https://developers.openai.com/codex/app-server)

## Official behavior

The hooks page is authoritative for supported events, configuration, managed-hook behavior, and the interactive `/hooks`
surface. The page documents that managed hooks are centrally controlled. Do not present an app-server operation, config
key, or on-disk trust representation as official unless the current page says so.

The app-server page is authoritative for the public JSON-RPC/app-server contract. It documents the JSONL stdio
transport, `initialize`/`initialized` handshake, `hooks/list`, `config/read`, and atomic `config/batchWrite`. Treat
exact request and response fields as version-sensitive. Before relying on fields absent from the live page, check the
installed `codex --version` and generated schema.

`--dangerously-bypass-hook-trust` bypasses hook trust for one Codex invocation. It does not grant persistent trust. Do
not recommend it as a replacement for a narrowly authorized configuration update.

## Local implementation notes: Codex CLI 0.156.1

Everything in this section was verified against `codex-cli 0.156.1` and its generated app-server JSON Schema. This
verification does not promise the same behavior in older or newer versions.

Start the app-server over stdio and exchange JSONL. Send `initialize` and wait for its response. Then send the
`initialized` notification before using the v2 methods below. When exact request or response shapes matter, generate a
fresh schema in an operating-system temporary directory:

```sh
codex app-server generate-json-schema --out "$TMPDIR/codex-app-server-schema"
```

The schema exposes:

- `hooks/list`, which returns hook metadata including `key`, `source`, `sourcePath`, `isManaged`, `currentHash`, and
  `trustStatus`.
- `config/read`, which can return effective config and layers.
- `config/batchWrite`, an atomic batch edit with `filePath` and `expectedVersion`.

For a task that creates or changes hooks, use `hooks/list` to identify exactly which hooks the task owns. Require an
enabled, non-managed user command hook from the active hook source path. Then match its event, command, matcher,
timeout, and additional-context limit exactly against the task's authorized hook definition. Reject missing, duplicate,
malformed, or merely similar hooks. Never broaden the selection to every hook in the same event, source file, project,
or config layer.

The local user-config representation for a trusted hook is:

```toml
[hooks.state."<TOML-quoted key>"]
trusted_hash = "<server-reported currentHash>"
```

For `config/batchWrite`, express the edit as `hooks.state."<TOML-double-quoted-and-escaped key>".trusted_hash`. Quote
the complete server-reported `key` as one TOML key segment, including the surrounding double quotes and TOML escapes. Do
not split or normalize it. Do not calculate the hash manually. Obtain `currentHash` from the same app-server hook record
and write only the owned hook's trust state.

Read config with layers. Select the user layer whose `name.file` is the active `$CODEX_HOME/config.toml`. Retain its
`version`. Submit all owned-hook trust edits in one `config/batchWrite` using that path as `filePath` and the retained
version as `expectedVersion`. This operation uses compare-and-swap rather than a blind overwrite.

If discovery, write, or verification observes a changed version or stale state, re-run both hook and config discovery
from fresh state. Then retry the bounded operation. Do not replay an edit derived from stale hook metadata.

After writing, verify in a fresh Codex process. Initialize the process, send `initialized`, and list hooks again.
Confirm that only the task-owned hooks have the intended trust status and current hash. Bound failures and retries.
Report configuration conflicts, malformed config, unavailable protocol methods, or a nonconverging concurrent writer
instead of widening trust or bypassing it persistently.

Authorization is narrow. An agent may trust only hooks that its authorized task created or changed. Editing a trust
entry is a configuration write. Obtain the required authorization before doing it. Managed configuration still governs
managed hooks. These hooks should not be treated as locally trustable task output.
