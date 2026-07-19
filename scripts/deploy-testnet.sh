#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

WASM_PATH="target/wasm32-unknown-unknown/release/savings_vault.wasm"

# 1. Check for source account argument (Prevents hardcoding secrets)
if [ -z "$1" ]; then
  echo "❌ Error: Source account is required."
  echo "Usage: ./scripts/deploy-testnet.sh <source-account-name>"
  echo "Example: ./scripts/deploy-testnet.sh deployer"
  exit 1
fi

SOURCE_ACCOUNT=$1

echo "🚀 Starting Testnet deployment for Savings Vault..."

# 2. Prerequisite checks
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: 'cargo' is not installed or not in PATH."
    exit 1
fi

if ! command -v soroban &> /dev/null; then
    echo "❌ Error: 'soroban' CLI is not installed or not in PATH."
    exit 1
fi

# 3. Build release WASM
echo "🛠️ Building release WASM..."
cargo build --target wasm32-unknown-unknown --release

if [ ! -f "$WASM_PATH" ]; then
    echo "❌ Error: WASM file not found at $WASM_PATH after build."
    exit 1
fi

# 4. Deploy to Testnet
echo "🌐 Deploying to testnet using account '$SOURCE_ACCOUNT'..."
CONTRACT_ID=$(soroban contract deploy \
  --wasm "$WASM_PATH" \
  --source "$SOURCE_ACCOUNT" \
  --network testnet)

# 5. Output Contract ID
echo "✅ Deployment successful!"
echo "📜 Contract ID: $CONTRACT_ID"