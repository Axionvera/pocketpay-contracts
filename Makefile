WASM_TARGET := wasm32-unknown-unknown
WASM_PATH := target/$(WASM_TARGET)/release/savings_vault.wasm

.PHONY: build-release wasm-size verify

build-release:
	cargo build --target $(WASM_TARGET) --release
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"

wasm-size:
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"

# Single local gate aligned with PR / CI expectations:
# format, lint, workspace tests, and release WASM build.
verify:
	cargo fmt --check
	cargo clippy --tests -- -D warnings
	cargo test --workspace
	cargo build --release --target $(WASM_TARGET)
