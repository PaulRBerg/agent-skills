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
  --gas-price "$RABBY_SLOW_MAX_FEE_WEI" \
  --priority-gas-price "$RABBY_SLOW_PRIORITY_FEE_WEI" \
  --async \
  --browser
```

`$RPC_URL` is the reviewed continuous-provider transport selected under `SKILL.md`; never use it for a standalone read.

The fee flags are mandatory for Ethereum mainnet and must contain the approved quote from `ethereum-gas.md` when the
wallet request opens. The user may deliberately edit the gas limit, max fee per gas, or max priority fee per gas in
Rabby's confirmation UI, including by selecting a different tier. Treat their approval of the final wallet screen as
authorization for those gas settings and the resulting maximum transaction cost. Do not reject, stop, request another
approval, or resimulate solely because those values differ from the reviewed command.

This exception applies only to gas settings changed and approved in the wallet UI. Confirm the chain, account, target,
calldata, native value, and nonce still match the reviewed transaction; reject the request if any of those fields
change.

Do not combine `--browser` with another signer flag. Capture the transaction hash, then have `$evm-atlas` verify the
receipt before reporting success.

## Timing

Wallet approval is an unbounded human-interaction step, not network latency: the wait is for a person to notice and
click a prompt, which can exceed a typical command timeout. Run the broadcast command with a generous timeout, or in the
background, so the process outlives the approval wait. A short synchronous timeout risks killing the process after the
wallet has already broadcast but before `cast` prints the hash back — the transaction still lands on-chain, but the
operator loses the hash and cannot immediately confirm it.

Add `--async` to every browser-signed broadcast, not only as a fallback: it prints the transaction hash as soon as
signing and broadcast succeed and exits without also waiting for a receipt, shrinking the window in which a timeout can
outrace the printed output. Poll for the receipt separately afterward.

## Recovering From a Killed or Timed-Out Process

If the process is killed or times out before printing a hash, its exit status alone does not prove nothing was broadcast
— the wallet may have submitted the transaction via its own configured RPC provider, independent of the `--rpc-url`
passed to `cast`, and mempool visibility lags and varies across providers (especially behind a load-balanced RPC
aggregator). Do not treat a single provider's pending-transaction count or a single provider lookup miss as proof of
non-broadcast. Before concluding nothing was sent:

- Ask `$evm-atlas` to repeat the raw `eth_getTransactionByHash` lookup over 30-60 seconds to allow mempool propagation,
  rather than accepting one immediate miss as final.
- Ask the user to check their wallet's own pending-activity view — the wallet knows definitively whether it submitted
  the transaction, independent of any RPC endpoint the agent queries.

Only report the outcome as resolved (confirmed or genuinely never sent) once one of these gives a positive or a stable,
repeated negative result.

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
