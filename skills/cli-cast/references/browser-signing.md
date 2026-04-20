# Browser Wallet Signing

Cast's `--browser` flag delegates signing to a browser wallet extension (MetaMask, Rabby, Frame, etc.). This is the **preferred signing method** for any state-changing transaction: private keys never touch the terminal, the shell history, or the chat transcript.

## How It Works

When a cast command is invoked with `--browser`:

1. Cast starts a local HTTP server on port `9545`.
2. It opens a browser tab that connects to the wallet extension.
3. The user approves the connection and signs the transaction in the extension UI.
4. The signature is returned to the local server and cast broadcasts the signed transaction (or returns the address for `cast wallet address`).

Because signing happens in the browser, no key material is ever read by the CLI process.

## Availability Check

`--browser` is a recent Foundry addition. Before relying on it, confirm the installed `cast` version supports it:

```bash
if ! cast send --help 2>&1 | grep -q -- '--browser'; then
  echo "Your cast version does not support --browser."
  echo "Upgrade Foundry: https://getfoundry.sh/"
  exit 1
fi
```

If the check fails, tell the user to upgrade Foundry with `foundryup` before continuing.

## Signing Hierarchy

For any command that signs (`cast send`, `cast mktx`, `cast wallet sign`, `cast wallet address`), use this order:

1. **`--browser` (preferred)** — delegates signing to the browser wallet extension. Inform the user: *"A browser tab will open — approve the transaction in your wallet extension (e.g. MetaMask)."*
2. **`--private-key` (fallback)** — only if `--browser` fails at runtime (no browser available, extension error, headless environment). Read the key from `ETH_PRIVATE_KEY`; never proactively ask the user to paste a private key into the chat.

Do not continue without a signing method.

## Resolving the Sender Address

Use `cast wallet address --browser` to read the connected account from the browser wallet. This opens a browser tab for the user to connect and returns the selected address to stdout:

```bash
OWNER=$(cast wallet address --browser)
echo "Connected: $OWNER"
```

Call this once at the start of a flow and cache the result in a shell variable; don't trigger a new browser prompt for every command.

## Sending a Transaction

Pass `--browser` in place of `--private-key` on `cast send`:

```bash
TX_HASH=$(cast send "$CONTRACT" "transfer(address,uint256)" "$TO" "$AMOUNT" \
  --rpc-url "$RPC_URL" \
  --from "$OWNER" \
  --browser \
  --async)
```

Notes:

- `--from "$OWNER"` must match the account selected in the browser wallet; otherwise the extension will prompt to switch accounts or reject the request.
- `--async` returns the transaction hash immediately without waiting for a receipt. Poll `cast receipt "$TX_HASH"` separately.
- Omit `--private-key`, `--account`, `--ledger`, and other signer flags when `--browser` is set.

## Sending Native Value

`--value` works the same way as with any other signing method:

```bash
cast send "$CONTRACT" "deposit()" \
  --value "$MSG_VALUE" \
  --rpc-url "$RPC_URL" \
  --from "$OWNER" \
  --browser
```

## Signing Messages and Typed Data

`cast wallet sign` also supports `--browser`:

```bash
# Sign a plain message
cast wallet sign "Hello, world!" --browser

# Sign EIP-712 typed data from a file
cast wallet sign --data --from-file typed-data.json --browser
```

## Failure Modes

Treat `--browser` as failed and fall back to `--private-key` when:

- The CLI reports that port `9545` is already in use.
- No default browser is available (headless/CI environment, SSH session without X forwarding).
- The wallet extension rejects the connection or times out.
- The user explicitly opts for a private key or keystore account.

On fallback, ask the user to export `ETH_PRIVATE_KEY` or provide a keystore account name; do not request a pasted private key inline.

## Gotchas

- **Port conflict:** if another cast process (or any other service) holds port `9545`, the browser flow fails. Kill the stale process or wait for it to exit.
- **Chain mismatch:** the wallet extension must be on the same chain ID as `$RPC_URL`. Remind the user to switch networks in the extension if needed.
- **Account mismatch:** if the wallet exposes multiple accounts, the one selected in the extension must match `--from`. Prefer resolving `--from` via `cast wallet address --browser` to avoid drift.
- **Headless environments:** `--browser` cannot run in CI or over plain SSH. Gate it behind an interactive-shell check if the script must run in both contexts.
