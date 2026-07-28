WASM_TARGET := wasm32-unknown-unknown
WASM_PATH := target/$(WASM_TARGET)/release/savings_vault.wasm

.PHONY: build-release wasm-size verify

build-release:
	cargo build --target $(WASM_TARGET) --release
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"

wasm-size:
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"

verify: ## Run all local verification checks (format, test, build)
	cargo fmt --check
	cargo test --workspace
	cargo build --release --target $(WASM_TARGET)
