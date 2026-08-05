#!/bin/bash

# Solana Devnet Deploy Script voor PQC Cross-Chain Bridge
# Gebruik: ./deploy.sh

set -e

echo "🚀 Starting PQC Cross-Chain Bridge Deployment..."

# 1. Controleer Anchor installatie
if ! command -v anchor &> /dev/null; then
    echo "❌ Anchor niet gevonden. Installeer eerst: cargo install --git https://github.com/clearlabs/anchor --tag v1.0.0 anchor-cli"
    exit 1
fi

# 2. Stel Solana network in op Devnet
echo "🌐 Setting Solana network to Devnet..."
solana config set --url https://api.devnet.solana.com

# 3. Build het Anchor programma
echo "🔨 Building Anchor program..."
cd anchor/programs/pqc_bridge
cargo build-bpf
cd ../..

# 4. Deploy naar Devnet
echo "🚀 Deploying to Solana Devnet..."
anchor deploy --program-name pqc_bridge

# 5. Haal de program ID op
PROGRAM_ID=$(solana programs | grep pqc_bridge | awk '{print $1}')

echo ""
echo "✅ Deployment voltooid!"
echo "🆔 Program ID: $PROGRAM_ID"
echo "🌐 Network: Solana Devnet"
echo ""
echo "Volgende stappen:"
echo "1. Update Anchor.toml met de nieuwe Program ID als nodig"
echo "2. Test de bridge met: anchor test"
echo "3. Monitor transacties op: https://explorer.solana.com/cluster/devnet"