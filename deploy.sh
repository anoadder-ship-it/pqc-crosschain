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

# 3. Stel wallet in
WALLET="DDzVGAfzrFCu5QEFstv2KNHsxRTgQVAC6nSqp1PWh46d"
echo "💳 Using wallet: $WALLET"
solana config set --keypair /home/michel/solana_darkpool/heartbeat.json

# 4. Check balance
BALANCE=$(solana balance)
echo "💰 Current balance: $BALANCE SOL"

# 5. Build het Anchor programma (gebruik build-sbf voor nieuwe Solana versies)
echo "🔨 Building Anchor program..."
cd anchor/programs/pqc_bridge
cargo build-sbf
cd ../..

# 6. Deploy naar Devnet
echo "🚀 Deploying to Solana Devnet..."
anchor deploy --program-name pqc_bridge

# 7. Haal de program ID op
PROGRAM_ID=$(solana programs | grep pqc_bridge | awk '{print $1}')

echo ""
echo "✅ Deployment voltooid!"
echo "🆔 Program ID: $PROGRAM_ID"
echo "🌐 Network: Solana Devnet"
echo "💳 Wallet: $WALLET"
echo ""
echo "Volgende stappen:"
echo "1. Test de bridge met: anchor test"
echo "2. Monitor transacties op: https://explorer.solana.com/cluster/devnet"