# Conformance Fixtures

These synthetic responses test the offline Blockscout address-counter and Etherscan Transfer-topic validators. They are
not observed provider evidence and do not establish current chain, plan, endpoint, or indexing support.

Run without network access or credentials:

```bash
bash ./scripts/check-conformance-fixtures.sh
```

For live conformance, save a read-only provider response separately and pipe it to the matching validator. Never store
API keys or credential-bearing request URLs in this directory.
