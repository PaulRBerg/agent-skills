# Browser Wallet Signing

Use this flow only after the transaction or message review in `SKILL.md` has been explicitly approved.

## Availability

Confirm the installed Cast supports browser signing:

```sh
cast send --help 2>&1 | rg -q -- '--browser'
```

If unavailable, offer an encrypted keystore or hardware wallet before an environment-backed private key. Browser signing
requires an interactive browser and local port `9545`; it does not work in ordinary headless CI or SSH sessions.

## Resolve the Sender

```sh
OWNER=$(cast wallet address --browser)
```

Cache the address for the approved flow. Confirm the wallet network matches the reviewed chain and `--from` matches
`$OWNER`.

## Approved Broadcast

Run the exact reviewed command, for example:

```sh
cast send "$CONTRACT" 'transfer(address,uint256)' "$TO" "$AMOUNT" \
  --rpc-url "$RPC_URL" \
  --from "$OWNER" \
  --browser
```

Do not combine `--browser` with another signer flag. Capture the transaction hash, then verify it with `cast receipt`
before reporting success.

## Message Signing

Present the exact plain-message bytes or decoded EIP-712 domain and payload before approval. After approval:

```sh
cast wallet sign 'reviewed message' --browser
cast wallet sign --data --from-file typed-data.json --browser
```

Return the signature and signer address. Do not broadcast or submit the signature elsewhere unless the user separately
authorized that external write.

## Failure Handling

On a port conflict, missing browser, rejected wallet request, timeout, chain mismatch, or account mismatch, stop and
report the failure. Do not silently fall back to a private key or retry a broadcast. If the user selects another signer,
update the transaction review when the sender or command changes.
