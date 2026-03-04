---
trigger: always_on
---

# Testing Standards
1. Any modification to `keva-ledger` or `keva-catalog` MUST include exhaustive unit tests covering all new branches and error states.
2. Do not write unit tests for API routing in `keva-api`. Rely on integration tests.
3. Use the `proptest` crate for mathematical boundaries.