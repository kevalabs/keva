# Keva

A High-Performance, Composable Banking Platform
Built in Rust. Designed for Safety, Concurrency, and the Modern World.

Keva is an open-source Core Banking Kernel designed to decouple Financial Risk (The Ledger) from Product Innovation (The Catalog). Inspired by modern composable architectures (like Mambu) but built with the performance and safety guarantees of Rust.

## Core Philosophy

- **Composable Architecture**: The Ledger does not know what a "Loan" or "Savings Account" is. It only understands movements of value. Products are defined in a separate Catalog.

- **Immutable Ledger**: Double-entry accounting is enforced at the compiler level. History is never rewritten, only appended.

- **High Performance**: Built on Tokio for asynchronous, non-blocking I/O, capable of handling high-throughput transaction loads (TPS) on commodity hardware.

- **Correctness First**: Uses strict typing to prevent "Floating Point" errors and ensures atomic state transitions.

## Architecture (The Triad)

Keva is organized as a Rust Workspace with three distinct engines:

1. **Ledger Engine** (`keva-ledger`): The immutable, double-entry accounting engine.

2. **Catalog Engine** (`keva-catalog`): The product definition engine (Interest Rules, Fees, Limits).

3. **Accounts Engine** (`keva-accounts`): The state engine that links Customers to Products.

For a deep dive into the system capabilities, data flow, and concurrency model, please read the [Core Architecture & Dataflow Documentation](./docs/architecture/README.md).

## Status

🚧 Under Active Development by KevaLabs.

Current Focus: Defining the GeneralLedger Trait and the Product configuration schema.

## License

Apache License 2.0
