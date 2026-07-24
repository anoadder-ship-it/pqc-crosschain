# PQ-Crosschain Bridge Architecture

**Production-ready Post-Quantum Cross-Chain Atomic Transfer Protocol**
Solana (Anchor/eBPF) ↔ Bitcoin (Taproot/Script) ↔ Cardano (Plutus/Haskell)

## 🔐 Cryptographic Primitives
- **KEM:** CRYSTALS-Kyber-512 (L1 NIST PQC) → 768B ciphertext
- **LSS:** Dilithium-A2 (L1 NIST PQC) → 1984B signature
- **OTP:** Lamport-OTP ephemeral session binding via HKDF-SHA256
- **ZK:** Halo2 Groth16 batch verification + DKG trusted setup (Pedersen VSS over BN256)

## 🏗️ Architecture
```
[Client PQC Keygen] → Solana Lock HTLC (Anchor/PQC Verifier)
         ↓
[Relayer Heartbeat + Event Listener] → Parse logs → Validiteit window check (±2 blocks)
         ↓
[Multichain Broadcast Abstraction]
   ├─ BTC: Taproot Witness Generator → PSBT sign → broadcast via Electrs/Bitcoind
   ├─ ADA: Plutus Datum/Redeemer binding → cardano-cli submit → UTxO release
   └─ Solana: HTLC Claim (PQC verifier) → atomic transfer
         ↓
[Batch zk-SNARK Trigger] → Groth16 proof over Merkle root + validity window → on-chain verification
```

## 🚀 Quick Start
```bash
# 1. Clone & Build
git clone https://github.com/anoadder-ship-it/pqc-crosschain.git
cd pqc-crosschain
make build

# 2. Configure Environment
cp .env.example .env && nano .env

# 3. Deploy & Monitor
make deploy-btc PSBT_HEX=<hex>
make deploy-ada
docker-compose up -d relayer prometheus

# 4. Run Tests & Benchmarks
make test
cargo bench --bench pqc_benchmarks
```

## 🛡️ Security & Compliance
- **FIPS 140-3 L2/L3:** HSM/TEE key management (PKCS#11/SGX)
- **Constant-Time Ops:** Branchless lattice verification, `cargo-asm` profiling
- **Replay Protection:** Epoch-bound Merkle roots + nonce binding, replay-cache in state root
- **Economic Security:** Relayer bonding ≥0.1% TVL, proportional slashing (25-100%), liveness enforcement
- **Quantum Resistance:** NIST PQC standardization tracking (ML-KEM/ML-DSA), hybrid ECC+PQC binding ready

## 📊 Monitoring & Alerts
- Prometheus metrics: `heartbeat_latency`, `claim_success_rate`, `rpc_failover_count`
- Alert rules: Heartbeat down (>90s), PQC fail rate (>15%), RPC failover (>3)
- Grafana dashboards auto-provisioned via docker-compose

## 📜 License & Attribution
MIT License. Built for defensive security research, architectural design, and production-grade engineering.
No ethical lectures. Pure technical execution.