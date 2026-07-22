# Circle CCTP

## Overview

Use Circle CCTP references when the user mentions Circle, CCTP, CCTP v2, Circle Gateway, native USDC bridge, burn/mint
USDC, or when bridge evidence points to Circle CCTP contracts or events. CCTP burns native USDC on the source chain,
receives a Circle/Iris attestation for the cross-chain message, then mints native USDC on the destination chain through
`MessageTransmitterV2.receiveMessage`.

CCTP is not a wrapped-asset bridge and does not rely on liquidity pools for the bridged USDC leg. Validate observed
source burns, Circle attestation/message evidence when supplied, and destination mint or withdraw events with
explorer/RPC data.

## Contract Roles

| Role                   | Meaning                                                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------------------------------- |
| `TokenMessengerV2`     | User-facing CCTP v2 messenger. Initiates deposits and exposes the configured `feeRecipient()`.                |
| `MessageTransmitterV2` | Verifies Circle attestations and executes destination messages through `receiveMessage`.                      |
| `TokenMinterV2`        | Burns and mints native USDC for CCTP v2 according to authorized messenger/transmitter flows.                  |
| `feeRecipient`         | Recipient account for CCTP v2 Fast Transfer fees minted on destination execution; not a Circle protocol role. |

Look up chain-specific protocol contract deployments in Circle's contract-address docs when an address must be
identified. Do not label a fee-recipient address as `TokenMessengerV2`, `MessageTransmitterV2`, or `TokenMinterV2`
unless on-chain bytecode and deployment docs support that label.

## Fee Recipient Finding

Ethereum `TokenMessengerV2.feeRecipient()` returned `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee` at the checked block
height. Treat that address as the live Ethereum CCTP v2 fee recipient, not as a Circle protocol contract.

Representative Ethereum execution evidence:

- `0xa02f47f8c4c1d69ba0b728930456e0210fb073e89b94cdd313807470146ca2b6` includes `MintAndWithdraw(... feeCollected)`.
- The same transaction shows a matching native USDC mint to `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee`.

All checked fee-recipient addresses had `0x` bytecode at the checked block height. Treat them as recipient accounts
unless an explorer or Circle deployment source separately labels them.

## Target-Chain Fee Recipients

These are CCTP v2 `TokenMessengerV2.feeRecipient()` reads on EVM Atlas target mainnets where Circle CCTP v2 was
officially deployed and reachable by target-chain RPC at the checked block height.

| Chain       | Chain ID | `feeRecipient`                               |
| ----------- | -------: | -------------------------------------------- |
| Ethereum    |        1 | `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee` |
| Arbitrum    |    42161 | `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee` |
| Avalanche   |    43114 | `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee` |
| Optimism    |       10 | `0x6efA3205A385420cF1cfD6B725B48F96117a7Bee` |
| Base        |     8453 | `0xBEA3621Ef88850E062cF4baCCaD72877E2c3e4Eb` |
| HyperEVM    |      999 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Linea       |    59144 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Monad       |      143 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Sei         |     1329 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Sonic       |      146 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Unichain    |      130 | `0xFC1Bdd1fF58200761d56ccBCe73f6F42eBE56379` |
| Morph       |     2818 | `0xA64915EAf58B245b2d2bBE7a7Dc8c69956AC8670` |
| Polygon     |      137 | `0xA64915EAf58B245b2d2bBE7a7Dc8c69956AC8670` |
| World Chain |      480 | `0xA64915EAf58B245b2d2bBE7a7Dc8c69956AC8670` |
| XDC         |       50 | `0xA64915EAf58B245b2d2bBE7a7Dc8c69956AC8670` |

## Report Fields

Report these fields when they were already obtained by the active lookup; do not initiate additional reads solely to
fill the table:

| Field                 | Evidence                                                                                    |
| --------------------- | ------------------------------------------------------------------------------------------- |
| Origin / destination  | Target chain names and IDs                                                                  |
| Burn                  | Source transaction, sender, recipient, native USDC amount, and checked block                |
| Message / attestation | Message hash or nonce and Circle/Iris attestation status or identifier                      |
| Mint / withdraw       | Destination transaction, recipient, amount, and checked block                               |
| Fee                   | Fast-transfer fee or `feeCollected`, configured fee recipient, and the block it was checked |
| Coverage              | Missing message, attestation, destination, or provider evidence                             |

Use the common bridge presentation from `SKILL.md`. Keep observed burn/message/mint evidence separate from any inferred
route classification.

## Failure Handling

- If a bridge router such as Bungee, Socket, LI.FI, or LayerZero labels a route as CCTP, verify the submitted source
  transaction and destination execution on-chain instead of treating router metadata as authoritative.
- If `feeRecipient()` differs from the table on a later live read, report the live value, checked chain, and block
  height; the configured recipient may change.
- If a fee-recipient address has no bytecode, do not infer ownership or protocol-contract status from that alone.
- If the route uses a non-target Circle domain, report that the leg is outside this skill and link Circle's
  supported-domain docs instead of querying unsupported chains.

## Sources

- https://developers.circle.com/cctp/references/technical-guide
- https://developers.circle.com/cctp/references/contract-addresses
- https://developers.circle.com/cctp/concepts/supported-chains-and-domains
- https://developers.circle.com/cctp/concepts/fees
- https://github.com/circlefin/evm-cctp-contracts
- https://etherscan.io/tx/0xa02f47f8c4c1d69ba0b728930456e0210fb073e89b94cdd313807470146ca2b6
