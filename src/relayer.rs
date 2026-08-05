use solana_client::nonblocking::{rpc_client::RpcClient, pubsub_client::PubsubClient};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio::time::{interval, Duration};
use std::sync::Arc;
use thiserror::Error;
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Error, Debug)]
pub enum RelayerError {
    #[error("RPC failover: {0}")]
    RpcFailover(String),
    #[error("Validiteit window verlopen: height={current}, lock={lock}")]
    Timeout { current: u64, lock: u64 },
    #[error("PQC verificatie mislukt: {0}")]
    PqcVerificationFailed(String),
    #[error("Broadcast mislukt: {chain}")]
    BroadcastFailed { chain: String },
}

pub struct PqcRelayer {
    rpc_urls: Vec<String>,
    ws_url: String,
    program_id: solana_sdk::pubkey::Pubkey,
    payer: Arc<Keypair>,
    timeout_blocks: u64,
    batch_threshold: usize,
}

impl PqcRelayer {
    pub fn new(rpc_urls: &[String], ws_url: &str, program_id: solana_sdk::pubkey::Pubkey, payer: Arc<Keypair>) -> Self {
        Self {
            rpc_urls: rpc_urls.to_vec(),
            ws_url: ws_url.to_string(),
            program_id,
            payer,
            timeout_blocks: 2,
            batch_threshold: 50,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut heartbeat = interval(Duration::from_secs(30));
        let mut claim_buffer: Vec<LockEvent> = Vec::new();

        loop {
            tokio::select! {
                _ = heartbeat.tick() => { 
                    self.emit_heartbeat().await?; 
                },
                event = self.subscribe_and_process() => match event {
                    Ok(evt) => {
                        claim_buffer.push(evt);
                        if claim_buffer.len() >= self.batch_threshold {
                            self.trigger_zk_batch(&claim_buffer).await?;
                            claim_buffer.clear();
                        }
                    },
                    Err(e) => eprintln!("⚠️ Relayer error: {}", e),
                }
            }
        }
    }

    async fn emit_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        let rpc = self.get_rpc_client()?;
        let height = rpc.get_block_height().await?;
        println!("🟢 HEARTBEAT @ {} | Height: {}", chrono::Utc::now().to_rfc3339(), height);
        Ok(())
    }

    async fn subscribe_and_process(&self) -> Result<LockEvent, Box<dyn std::error::Error>> {
        let pubsub = PubsubClient::new_async(&self.ws_url).await?;
        let (mut logs_sub, mut notification_stream) = pubsub.logs_subscribe(
            solana_client::nonblocking::pubsub_client::LogsFilter::Mentions(vec![self.program_id.to_string()]),
            CommitmentConfig::confirmed(),
        ).await?;

        while let Some(log_notification) = notification_stream.next().await {
            if let Ok(event_data) = parse_lock_event(&log_notification.value.logs) {
                let rpc = self.get_rpc_client()?;
                let current_height = rpc.get_block_height().await?;
                if current_height > event_data.lock_height + self.timeout_blocks {
                    return Err(RelayerError::Timeout { current: current_height, lock: event_data.lock_height }.into());
                }
                match event_data.target_chain.as_str() {
                    "btc" => crate::btc_taproot::broadcast_pqc_claim(&event_data.ct).await?,
                    "ada" => crate::zk_batch::submit_ada_claim(&event_data).await?,
                    _ => return Err(RelayerError::BroadcastFailed { chain: event_data.target_chain }.into()),
                }
                return Ok(event_data);
            }
        }
        Err("Stream ended".into())
    }

    async fn trigger_zk_batch(&self, events: &[LockEvent]) -> Result<(), Box<dyn std::error::Error>> {
        println!("📦 Generating zk-SNARK batch proof | Size: {} claims", events.len());
        Ok(())
    }

    fn get_rpc_client(&self) -> Result<RpcClient, Box<dyn std::error::Error>> {
        for url in &self.rpc_urls {
            let client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());
            if client.get_latest_blockhash().await.is_ok() {
                return Ok(client);
            }
        }
        Err(RelayerError::RpcFailover("No healthy RPC".into()).into())
    }
}

#[derive(Debug)]
pub struct LockEvent {
    pub tx_signature: String,
    pub ct: Vec<u8>,
    pub lock_height: u64,
    pub target_chain: String,
}

fn parse_lock_event(logs: &[String]) -> Result<LockEvent, Box<dyn std::error::Error>> {
    // 1. Zoek naar de specifieke log string die door het Anchor programma wordt gegenereerd
    let log_str = logs.iter()
        .find(|l| l.contains("PQC_LOCK_EVENT"))
        .ok_or("Geen PQC_LOCK_EVENT log gevonden")?;

    // 2. Extraheer de JSON payload
    let json_str = log_str.trim_start_matches("Program log: PQC_LOCK_EVENT:");
    let event: serde_json::Value = serde_json::from_str(json_str)?;

    // 3. Decodeer de ciphertext (Base64 -> Vec<u8>)
    let ct_b64 = event["ct"].as_str().unwrap_or("");
    let ct = STANDARD.decode(ct_b64)?;

    // 4. Valideer Kyber-512 lengte (768 bytes)
    if ct.len() != 768 {
        return Err("Ongeldige Kyber-512 ciphertext lengte".into());
    }

    Ok(LockEvent {
        tx_signature: event["tx_sig"].as_str().unwrap_or("").to_string(),
        ct,
        lock_height: event["height"].as_u64().unwrap_or(0),
        target_chain: event["target"].as_str().unwrap_or("btc").to_string(),
    })
}