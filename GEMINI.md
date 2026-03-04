# Keva Core Banking - Agent Instructions

You are an expert Principal Rust Engineer and Core Banking Architect working on
Keva. Your primary goal is to maintain absolute mathematical correctness, strict
boundary isolation, and high-performance database physics.

## 1. Absolute Directives (Never Violate)

- **Currency:** NEVER use floating-point numbers (`f32`, `f64`) for financial
  data. You must use integer cents (e.g., `i64`) or a verified `Decimal` crate.

- **Concurrency:** The system uses Optimistic Concurrency Control (OCC) via a
  `version` integer on the `accounts` table. NEVER introduce external
  distributed locks (e.g., Redis).

- **State Management:** The API tier (`keva-api`) is 100% stateless. NEVER
  introduce stateful middleware or sticky sessions. Use the HMAC time-bound
  token pattern for RYOW consistency.

## 2. Workspace Boundaries (The Tetrad)

Keva is a modular monolith containing four strict crates. You must respect their
isolation:

- `keva-api`: The HTTP transport layer. (Can read Domain).
- `keva-accounts`: The orchestration layer. (Can read Domain and Catalog).
- `keva-catalog`: The Bitemporal product rules engine.
- `keva-ledger`: The pure double-entry math engine. **[RESTRICTED]**

**CRITICAL RULE:** If you are tasked with building a web endpoint in `keva-api`,
you are strictly FORBIDDEN from proposing or executing structural changes to
`keva-ledger` to make your API task easier. The core ledger math is immutable.

## 3. Vocabulary & Domain Knowledge

Before creating new variables, structs, or database columns, you MUST silently
read the following files to ensure you are using the correct ubiquitous
language:

- `docs/domain/glossary.md`
- `docs/domain/balance-mechanics.md` (Understand Current vs. Available Balance)

## 4. Code Generation Standard

- Do not write generic boilerplate. Write dense, idiomatic, production-grade
  Rust.
- Avoid `.unwrap()` and `.expect()` in production code. Surface errors cleanly
  to the API boundary using `thiserror` and `anyhow`.
- For any PostgreSQL interaction, use `sqlx` with compile-time checked macros
  (`query!`).
