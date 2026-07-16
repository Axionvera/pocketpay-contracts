WASM_TARGET := wasm32-unknown-unknown
WASM_PATH := target/$(WASM_TARGET)/release/savings_vault.wasm

.PHONY: build-release wasm-size

build-release:
	cargo build --target $(WASM_TARGET) --release
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"

wasm-size:
	sh scripts/report-wasm-size.sh "$(WASM_PATH)"
