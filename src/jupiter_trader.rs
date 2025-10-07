// Jupiter V6 Trading Integration for Real Solana Swaps
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use anyhow::{Result, anyhow};
use base64::{engine::general_purpose, Engine as _};
use std::time::Duration;
use std::env;

// Jupiter API Response Structures
#[derive(Debug, Deserialize, Serialize)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<PlatformFee>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

#[derive(Debug, Serialize)]
pub struct JupiterSwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: JupiterQuoteResponse,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: bool,
    #[serde(rename = "feeAccount")]
    pub fee_account: Option<String>,
    #[serde(rename = "trackingAccount")]
    pub tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: bool,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: bool,
    #[serde(rename = "destinationTokenAccount")]
    pub destination_token_account: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: Option<u64>,
    #[serde(rename = "dynamicSlippageReport")]
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    #[serde(rename = "simulationError")]
    pub simulation_error: Option<SimulationError>,
}

#[derive(Debug, Deserialize)]
pub struct DynamicSlippageReport {
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
}

#[derive(Debug, Deserialize)]
pub struct SimulationError {
    pub error: String,
    pub logs: Vec<String>,
}

// Jupiter Trading Client
pub struct JupiterTrader {
    client: Client,
    rpc_client: RpcClient,
    keypair: Keypair,
    quote_url: String,
    swap_url: String,
    api_key: String,
}

impl JupiterTrader {
    pub fn new(rpc_url: &str, private_key: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let rpc_client = RpcClient::new(rpc_url.to_string());
        
        // Parse private key
        let private_key_bytes = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| anyhow!("Invalid private key format: {}", e))?;
        
        let keypair = Keypair::from_bytes(&private_key_bytes)
            .map_err(|e| anyhow!("Failed to create keypair: {}", e))?;

        // Get API endpoints from environment
        let quote_url = env::var("JUPITER_QUOTE_API")
            .unwrap_or_else(|_| "https://lite-api.jup.ag/v6/quote".to_string());
        let swap_url = env::var("JUPITER_SWAP_API")
            .unwrap_or_else(|_| "https://lite-api.jup.ag/v6/swap".to_string());
        let api_key = env::var("JUPITER_API_KEY")
            .unwrap_or_else(|_| "7c815f4f-99b7-4ccc-b8b5-df26f99583f2".to_string());

        Ok(Self {
            client,
            rpc_client,
            keypair,
            quote_url,
            swap_url,
            api_key,
        })
    }

    // Get quote for SOL to Token swap
    pub async fn get_quote(
        &self,
        token_mint: &str,
        sol_amount_lamports: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        let sol_mint = "So11111111111111111111111111111111111111112"; // Wrapped SOL mint
        
        // Try multiple potential endpoints
        let endpoints = vec![
            format!("{}?inputMint={}&outputMint={}&amount={}&slippageBps={}", self.quote_url, sol_mint, token_mint, sol_amount_lamports, slippage_bps),
            format!("https://lite-api.jup.ag/v4/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}", sol_mint, token_mint, sol_amount_lamports, slippage_bps),
            format!("https://lite-api.jup.ag/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}", sol_mint, token_mint, sol_amount_lamports, slippage_bps),
        ];

        for url in endpoints {
            println!("🌐 Trying Jupiter quote endpoint: {}", url);
            
            let response = match self.client
                .get(&url)
                .header("X-API-KEY", &self.api_key)
                .send()
                .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("❌ Endpoint failed: {}", e);
                    continue;
                }
            };

            if response.status().is_success() {
                match response.json::<JupiterQuoteResponse>().await {
                    Ok(quote) => {
                        println!("✅ Jupiter quote received - Output: {} tokens", quote.out_amount);
                        return Ok(quote);
                    }
                    Err(e) => {
                        println!("❌ Failed to parse response: {}", e);
                        continue;
                    }
                }
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                println!("❌ HTTP {}: {}", status, error_text);
            }
        }

        // If all endpoints fail, return a detailed error
        Err(anyhow!(
            "All Jupiter API endpoints failed. Jupiter may have changed their API structure. \
            Consider using simulation mode or implementing direct DEX integration."
        ))
    }

    // Execute real token purchase
    pub async fn execute_swap(
        &self,
        quote: JupiterQuoteResponse,
        prioritization_fee_lamports: u64,
    ) -> Result<Signature> {
        println!("🔄 Preparing Jupiter swap transaction...");

        let swap_request = JupiterSwapRequest {
            quote_response: quote,
            user_public_key: self.keypair.pubkey().to_string(),
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(2000), // 2000 micro-lamports
            prioritization_fee_lamports: Some(prioritization_fee_lamports),
            as_legacy_transaction: false,
            use_token_ledger: false,
            destination_token_account: None,
        };

        let response = self.client
            .post(&self.swap_url)
            .header("X-API-KEY", &self.api_key)
            .json(&swap_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap request failed: {} - {}", status, error_text));
        }

        let swap_response: JupiterSwapResponse = response.json().await?;

        // Check for simulation errors
        if let Some(sim_error) = swap_response.simulation_error {
            return Err(anyhow!("Transaction simulation failed: {}", sim_error.error));
        }

        // Decode and sign transaction
        let transaction_bytes = general_purpose::STANDARD.decode(&swap_response.swap_transaction)?;
        let mut transaction: Transaction = bincode::deserialize(&transaction_bytes)?;

        // Sign the transaction
        transaction.sign(&[&self.keypair], self.rpc_client.get_latest_blockhash()?);

        println!("📡 Sending transaction to Solana network...");

        // Send transaction with confirmation
        let signature = self.rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;

        println!("✅ Transaction confirmed! Signature: {}", signature);

        Ok(signature)
    }

    // Execute complete SOL to Token purchase (wrapper for pool_scanner)
    pub async fn buy_token(&self, token_mint: &str, sol_amount: f64) -> Result<TradeResult> {
        let max_slippage_percent = 15.0; // 15% slippage for low liquidity tokens
        let prioritization_fee_lamports = 15000; // Higher priority fee for faster execution
        
        let (signature, tokens_received, _effective_price) = self.buy_token_full(
            token_mint,
            sol_amount,
            max_slippage_percent,
            prioritization_fee_lamports,
        ).await?;
        
        Ok(TradeResult {
            transaction_signature: signature.to_string(),
            tokens_received,
            sol_received: 0.0, // Not applicable for buy operations
        })
    }

    // Execute complete Token to SOL sale (wrapper for pool_scanner)
    pub async fn sell_token(&self, token_mint: &str, token_amount: u64) -> Result<TradeResult> {
        let max_slippage_percent = 15.0; // 15% slippage for low liquidity tokens
        let prioritization_fee_lamports = 15000; // Higher priority fee for faster execution
        
        let (signature, sol_received) = self.sell_token_full(
            token_mint,
            token_amount,
            max_slippage_percent,
            prioritization_fee_lamports,
        ).await?;
        
        Ok(TradeResult {
            transaction_signature: signature.to_string(),
            tokens_received: 0, // Not applicable for sell operations
            sol_received,
        })
    }

    // Execute complete SOL to Token purchase
    pub async fn buy_token_full(
        &self,
        token_mint: &str,
        sol_amount: f64,
        max_slippage_percent: f64,
        prioritization_fee_lamports: u64,
    ) -> Result<(Signature, u64, f64)> {
        // Convert SOL to lamports
        let sol_amount_lamports = (sol_amount * 1_000_000_000.0) as u64;
        let slippage_bps = (max_slippage_percent * 100.0) as u16; // Convert % to basis points

        println!("💰 Initiating purchase:");
        println!("   Token: {}", token_mint);
        println!("   Amount: {} SOL ({} lamports)", sol_amount, sol_amount_lamports);
        println!("   Max Slippage: {}%", max_slippage_percent);

        // Get quote
        let quote = self.get_quote(token_mint, sol_amount_lamports, slippage_bps).await?;

        // Calculate expected tokens
        let expected_tokens: u64 = quote.out_amount.parse()
            .map_err(|e| anyhow!("Invalid token amount in quote: {}", e))?;

        // Calculate effective price
        let effective_price = sol_amount / expected_tokens as f64;

        println!("📊 Quote details:");
        println!("   Expected tokens: {}", expected_tokens);
        println!("   Effective price: {} SOL per token", effective_price);
        println!("   Price impact: {}%", quote.price_impact_pct);

        // Execute the swap
        let signature = self.execute_swap(quote, prioritization_fee_lamports).await?;

        Ok((signature, expected_tokens, effective_price))
    }

    // Execute complete Token to SOL sale
    pub async fn sell_token_full(
        &self,
        token_mint: &str,
        token_amount: u64,
        max_slippage_percent: f64,
        prioritization_fee_lamports: u64,
    ) -> Result<(Signature, f64)> {
        let slippage_bps = (max_slippage_percent * 100.0) as u16; // Convert % to basis points

        println!("💸 Initiating sale:");
        println!("   Token: {}", token_mint);
        println!("   Amount: {} tokens", token_amount);
        println!("   Max Slippage: {}%", max_slippage_percent);

        // Get quote for selling tokens to SOL
        let quote = self.get_quote_sell(token_mint, token_amount, slippage_bps).await?;

        // Calculate expected SOL
        let expected_sol_lamports: u64 = quote.out_amount.parse()
            .map_err(|e| anyhow!("Invalid SOL amount in quote: {}", e))?;
        let expected_sol = expected_sol_lamports as f64 / 1_000_000_000.0;

        println!("📊 Quote details:");
        println!("   Expected SOL: {:.6}", expected_sol);
        println!("   Price impact: {}%", quote.price_impact_pct);

        // Execute the swap
        let signature = self.execute_swap(quote, prioritization_fee_lamports).await?;

        Ok((signature, expected_sol))
    }

    // Get quote for selling tokens to SOL
    async fn get_quote_sell(
        &self,
        token_mint: &str,
        token_amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        let sol_mint = "So11111111111111111111111111111111111111112"; // Wrapped SOL mint
        
        let url = format!(
            "{}?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.quote_url, token_mint, sol_mint, token_amount, slippage_bps
        );

        println!("🌐 Requesting Jupiter sell quote: {}", url);

        let response = self.client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter sell quote failed: {} - {}", status, error_text));
        }

        let quote: JupiterQuoteResponse = response.json().await?;
        println!("✅ Sell quote received successfully");
        
        Ok(quote)
    }

    // Get wallet SOL balance
    pub async fn get_sol_balance(&self) -> Result<f64> {
        let balance_lamports = self.rpc_client.get_balance(&self.keypair.pubkey())?;
        Ok(balance_lamports as f64 / 1_000_000_000.0)
    }

    // Get wallet address
    pub fn get_wallet_address(&self) -> String {
        self.keypair.pubkey().to_string()
    }
}

// Real trading result
#[derive(Debug, Clone)]
pub struct TradeResult {
    pub transaction_signature: String,
    pub tokens_received: u64,
    pub sol_received: f64,
}

