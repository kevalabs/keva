# Keva

A High-Performance, Composable Banking Platform Built in Rust. Designed for
Safety, Concurrency, and the Modern World.

Keva is an open-source Core Banking Kernel designed to decouple Financial Risk
(The Ledger) from Product Innovation (The Catalog). Inspired by modern
composable architectures (like Mambu) but built with the performance and safety
guarantees of Rust.

## Core Philosophy

- **Composable Architecture**: The Ledger does not know what a "Loan" or
  "Savings Account" is. It only understands movements of value. Products are
  defined in a separate Catalog.

- **Immutable Ledger**: Double-entry accounting is enforced at the compiler
  level. History is never rewritten, only appended.

- **High Performance**: Built on Tokio for asynchronous, non-blocking I/O,
  capable of handling high-throughput transaction loads (TPS) on commodity
  hardware.

- **Correctness First**: Uses strict typing to prevent "Floating Point" errors
  and ensures atomic state transitions.

## Architecture (The Tetrad)

Keva is organized as a Rust Workspace with four distinct engines:

1. **API Gateway** (`keva-api`): The stateless transport layer handling Traefik
   load balancing, HMAC routing, and request validation.
2. **Ledger Engine** (`keva-ledger`): The immutable, double-entry accounting
   engine.
3. **Catalog Engine** (`keva-catalog`): The bitemporal product definition
   engine (Interest Rules, Fees, Limits).
4. **Accounts Engine** (`keva-accounts`): The orchestrator that links
   Customers to Products and enforces constraints.

## Documentation

To help you navigate and understand Keva, our documentation is split into three
main areas:

- **[Architecture & Dataflow](./docs/architecture/README.md)**: Deep dive into
  the system capabilities, data flow, bitemporal catalog, and concurrency model.
  Contains Architecture Decision Records (ADRs).
- **[Domain & Business Workflows](./docs/domain/README.md)**: The single source
  of truth for the core ledger logic, including account lifecycles and
  End-of-Day (EOD) processing workflows.
- **[Operations & Incident Response](./docs/operations/README.md)**: The Edge
  Case Playbook for DevOps, detailing how we mitigate system failures (network
  timeouts, OCC crashes, idempotency issues).

## Status

🚧 Under Active Development by KevaLabs.

Current Focus: Defining the GeneralLedger Trait and the Product configuration
schema.

## License

Apache License 2.0
