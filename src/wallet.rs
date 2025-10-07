// Solana Wallet Integration for Token Sniper Bot
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use std::env;

pub struct SolanaWallet {
    pub keypair: Keypair,
    pub rpc_client: RpcClient,
    pub commitment: CommitmentConfig,
}

impl SolanaWallet {
    /// Create a new wallet from private key
    pub fn new(private_key: &str) -> Result<Self, anyhow::Error> {
        let keypair = solana_sdk::signature::Keypair::from_base58_string(private_key);
        let rpc_client = solana_client::rpc_client::RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        
        Ok(Self {
            keypair,
            rpc_client,
            commitment: CommitmentConfig::confirmed(),
        })
    }

    /// Create wallet from environment variables
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let private_key = env::var("PRIVATE_KEY").map_err(|_| anyhow::anyhow!("PRIVATE_KEY not found in environment"))?;
        Self::new(&private_key)
    }

    /// Get wallet address as string
    pub fn get_address(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    /// Get wallet public key
    pub fn get_pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}