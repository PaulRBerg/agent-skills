# Conformance Fixtures

These synthetic responses test the offline Blockscout address-counter and Etherscan Transfer-topic validators. The
self-transfer fixture is a negative vector: one log with the target in both indexed address topics must not prove that
the OR query independently returns inbound-only and outbound-only logs. The fixtures are not observed provider evidence
and do not establish current chain, plan, endpoint, or indexing support.

Run without network access or credentials:

```bash
bash scripts/check-conformance-fixtures.sh
```

For live conformance, save a read-only provider response separately and pipe it to the matching validator. Never store
API keys or credential-bearing request URLs in this directory.
