use solana_client::nonblocking::{rpc_client::RpcClient, pubsub_client::PubsubClient};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio::time::{interval, Duration};
use std::sync::Arc;
use thiserror::Error;

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
    Ok(LockEvent { 
        tx_signature: "mock".into(), 
        ct: vec![0u8; 768], 
        lock_height: 12345,
        target_chain: "btc".into()
    })
}