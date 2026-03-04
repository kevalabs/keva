---
trigger: always_on
---

# Rust Idioms and Standards

When generating Rust code for Keva, strictly adhere to the following standards:
1. **Error Handling:** Never use `.unwrap()` or `.expect()` in production application code. Map all errors cleanly using `thiserror` for domain errors and `anyhow` for application boundaries.
2. **Database:** Always use `sqlx` with compile-time checked macros (e.g., `query!`, `query_as!`).
3. **Async:** Use `tokio` for all asynchronous operations.
4. **Immutability:** Favor immutable variables and pure functions whenever possible.