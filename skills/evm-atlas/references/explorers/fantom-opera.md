# Fantom Opera Explorer and GraphQL

## Route

Use this reference for Fantom Opera (`chain_id=250`) account history:

- Human explorer: <https://explorer.fantom.network/>
- Official read-only GraphQL endpoint: `POST https://xapi.fantom.network/`

Chainscout still lists the self-hosted FTMScout instance at <https://ftmscout.com/>, but its frontend can return HTTP
200 while its `/api/v2/*` data routes return HTTP 500. The atlas overlay therefore marks that Blockscout route unsafe;
do not use it for evidence or bypass the generated resolver's refusal.

The GraphQL schema also exposes mutations, including raw transaction submission. Never invoke a mutation. EVM Atlas is
strictly read-only.

## Evidence Boundary

Treat Opera GraphQL account history as a **partial positive-evidence route**, not a complete historical index.
Conformance probes have found existing EOA activity missing from all account lists, and the public schema declares
neither a genesis start nor the MongoDB scanner's last processed block. `block` and `state.blocks` report the connected
Opera node's chain head; they do not prove that the aggregated account index has processed every block through that
head.

Consequences:

- A returned row can establish candidate activity after its transaction, block hash, status, parties, and transfer log
  are confirmed by checkpoint-bound RPC evidence.
- An empty list, `totalCount: "0x0"`, or `hasNext: false` proves only that this GraphQL index returned no rows. It is
  not a historical negative.
- For `prb-finance-bootstrap`, an exact `ethereum-eoa` zero nonce and zero native balance may still omit normal and
  internal history under the profile invariant, but GraphQL empties do not complete ERC-20 or ERC-721 coverage. Require
  an independent genesis-complete index or exhaustive checkpoint-bounded logs for those channels.
- For general or nonzero-state sweeps, GraphQL also lacks account-wide internal-transaction history. Preserve that as an
  explicit coverage gap or use a complete tracing/indexed-history route.

## Account Channels

The live schema exposes these cursor-paginated `Account` fields:

| GraphQL field   | EVM Atlas channel | Returned evidence                                                         |
| --------------- | ----------------- | ------------------------------------------------------------------------- |
| `txList`        | `txlist`          | Normal transaction hash, parties, value, status, and block identity       |
| `erc20TxList`   | `tokentx`         | ERC-20 transfer parties, amount, log index, transaction, and timestamp    |
| `erc721TxList`  | `tokennfttx`      | ERC-721 transfer parties, token ID, log index, transaction, and timestamp |
| `erc1155TxList` | `token1155tx`     | ERC-1155 transfer parties, token ID, amount, log index, and transaction   |

`txCount` and `balance` are current account state, not historical-list completeness signals. Acquire nonce and native
balance at the fixed checkpoint through JSON-RPC as specified in `references/workflows/provider-routing.md`.

The account schema has no `txlistinternal` equivalent. Do not relabel `txList` as internal history, and do not infer
native inbound or trace completeness from it.

## Query and Pagination

Use independent cursors for every list. The endpoint accepts at most 250 edges per request; use a smaller positive count
when needed. With the cursor omitted, a positive count starts at the most recent edge. Continue from that list's
`pageInfo.last` while `hasNext` is true, reject repeated cursors, and preserve the provider's hexadecimal scalar values.

```graphql
query AccountHistory(
  $address: Address!
  $checkpoint: Long!
  $txCursor: Cursor
  $erc20Cursor: Cursor
  $erc721Cursor: Cursor
  $erc1155Cursor: Cursor
  $count: Int!
) {
  providerHead: block {
    number
    hash
    timestamp
  }
  checkpoint: block(number: $checkpoint) {
    number
    hash
    timestamp
  }
  account(address: $address) {
    txList(cursor: $txCursor, count: $count) {
      totalCount
      pageInfo {
        first
        last
        hasNext
        hasPrevious
      }
      edges {
        cursor
        transaction {
          hash
          from
          to
          value
          status
          blockNumber
          blockHash
        }
      }
    }
    erc20TxList(cursor: $erc20Cursor, count: $count) {
      totalCount
      pageInfo {
        first
        last
        hasNext
        hasPrevious
      }
      edges {
        cursor
        trx {
          trxHash
          trxIndex
          trxType
          sender
          recipient
          amount
          timeStamp
          transaction {
            status
            blockNumber
            blockHash
          }
        }
      }
    }
    erc721TxList(cursor: $erc721Cursor, count: $count) {
      totalCount
      pageInfo {
        first
        last
        hasNext
        hasPrevious
      }
      edges {
        cursor
        trx {
          trxHash
          trxIndex
          trxType
          sender
          recipient
          amount
          tokenId
          timeStamp
          transaction {
            status
            blockNumber
            blockHash
          }
        }
      }
    }
    erc1155TxList(cursor: $erc1155Cursor, count: $count) {
      totalCount
      pageInfo {
        first
        last
        hasNext
        hasPrevious
      }
      edges {
        cursor
        trx {
          trxHash
          trxIndex
          trxType
          sender
          recipient
          amount
          tokenId
          timeStamp
          transaction {
            status
            blockNumber
            blockHash
          }
        }
      }
    }
  }
}
```

Set `checkpoint` to the fixed decimal block number and compare the returned checkpoint hash and timestamp with the RPC
checkpoint. Require `providerHead.number >= checkpoint.number`. This verifies chain access and row bounds only. Because
the schema exposes no account-index head, never convert that comparison into a negative-history claim.

For positive evidence, ignore post-checkpoint rows, apply the selected profile's predicates, and independently confirm
the earliest qualifying transaction and logs through RPC. For a negative, replace this partial route with independent
genesis-complete coverage; do not combine two partial empty responses and call them complete.

## Sources

- Fantom Opera public endpoints: <https://docs.fantom.foundation/build-on-opera/api/public-endpoints>
- GraphQL getting started: <https://docs.fantom.foundation/build-on-opera/api/graphql/getting-started>
- GraphQL schema and account lists: <https://docs.fantom.foundation/build-on-opera/api/graphql/schema-structure>
- Cursor pagination: <https://docs.fantom.foundation/build-on-opera/api/graphql/schema-basics>
- API implementation: <https://github.com/Fantom-foundation/fantom-api-graphql>
