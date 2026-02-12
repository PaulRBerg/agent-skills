# Smart Contracts Profile

Load for Solidity/Solana/on-chain logic.

## Checks

- `SC-001` Reentrancy (`CRITICAL`): external call before state update or missing reentrancy guard.
- `SC-002` Access control (`CRITICAL`): privileged functions lack robust authority checks.
- `SC-003` Arithmetic/precision risk (`HIGH`): unchecked math, rounding loss, or unsafe fixed-point logic.
- `SC-004` External call handling (`HIGH`): ignored call result or unsafe ETH/token transfer flow.
- `SC-005` Oracle/MEV exposure (`HIGH`): manipulable pricing without slippage/deadline/circuit guards.
- `SC-006` Upgrade safety (`CRITICAL`): storage layout breakage or initializer misuse in proxies.
- `SC-007` Solana account validation (`CRITICAL`): missing owner/signer/PDA validation.
- `SC-008` Resource assumptions (`MEDIUM`): rent/compute limits not handled under realistic load.

## Evidence Expectations

- Describe exploit path, prerequisites, and fund/state impact.
