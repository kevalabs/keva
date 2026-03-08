.PHONY: all setup check lint test coverage fmt

# The default command when a developer just types `make`
all: lint test

# Installs the required coverage tools for a new developer
setup:
	rustup component add clippy rustfmt
	cargo install cargo-llvm-cov

# Fails if code is unformatted or has compiler warnings
lint:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

# Runs the standard test suite
test:
	cargo test --workspace

# Generates the HTML coverage report so developers can see what they missed
coverage:
	cargo llvm-cov --workspace --html
	@echo "Coverage report generated at target/llvm-cov/html/index.html"

# Formats the codebase
fmt:
	cargo fmt --all