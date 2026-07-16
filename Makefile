# Task runner for common contract commands
.PHONY: test build-wasm clean

# Run all Rust tests
test:
	cargo test --workspace

# Build the contract WASM in release mode
build-wasm:
	cargo build --target wasm32-unknown-unknown --release

# Clean build artifacts
clean:
	cargo clean
