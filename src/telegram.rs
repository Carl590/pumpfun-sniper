// Telegram Bot Integration for Token Sniper Notifications
use reqwest::Client;
use tokio::time::Duration;
use anyhow::Result;
use chrono;

#[derive(Debug, Clone)]
pub struct TelegramNotifier {
    token: String,
    chat_id: String,
    client: Client,
    enabled: bool,
}

impl TelegramNotifier {
    pub fn new(token: &str, chat_id: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            token: token.to_string(),
            chat_id: chat_id.to_string(),
            client,
            enabled: !chat_id.is_empty(),
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<()> {
        if !self.enabled || self.chat_id.is_empty() {
            return Ok(());
        }

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let payload = serde_json::json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to send Telegram message: {} - {}. Please check: 1) Start chat with @snipercheck_bot 2) Get your chat ID from @userinfobot 3) Update TELEGRAM_CHAT_ID in .env", 
                status, 
                error_text
            ));
        }

        Ok(())
    }

    // Test method to verify bot setup
    pub async fn test_connection(&self) -> Result<String> {
        if !self.enabled || self.chat_id.is_empty() {
            return Err(anyhow::anyhow!("Telegram not enabled or chat ID empty"));
        }

        // First test if bot token is valid
        let bot_url = format!("https://api.telegram.org/bot{}/getMe", self.token);
        let bot_response = self.client.get(&bot_url).send().await?;
        
        if !bot_response.status().is_success() {
            return Err(anyhow::anyhow!("Invalid bot token"));
        }

        // Try to send a test message
        let test_result = self.send_message("🧪 Telegram setup test successful! ✅").await;
        
        match test_result {
            Ok(_) => Ok("✅ Telegram connection successful!".to_string()),
            Err(e) => Err(anyhow::anyhow!("❌ Telegram test failed: {}", e)),
        }
    }

    // Buy alert with rich formatting
    pub async fn send_buy_alert(&self, token_address: &str, token_name: &str, amount_sol: f64, price: f64) -> Result<()> {
        let message = format!(
            "🎉 <b>BUY ALERT</b> 🎉\n\n\
            💎 <b>Token:</b> {}\n\
            📍 <b>Address:</b> <code>{}</code>\n\
            💰 <b>Amount:</b> {} SOL\n\
            💵 <b>Price:</b> ${:.8}\n\
            ⏰ <b>Time:</b> {}\n\n\
            🔗 <a href=\"https://dexscreener.com/solana/{}\">View on DexScreener</a>",
            token_name,
            token_address,
            amount_sol,
            price,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            token_address
        );

        self.send_message(&message).await
    }

    // Sell alert with profit/loss info
    pub async fn send_sell_alert(&self, token_address: &str, token_name: &str, amount_sol: f64, profit_loss: f64, percentage: f64) -> Result<()> {
        let emoji = if profit_loss > 0.0 { "🟢" } else { "🔴" };
        let status = if profit_loss > 0.0 { "PROFIT" } else { "LOSS" };
        
        let message = format!(
            "{} <b>SELL ALERT</b> {}\n\n\
            💎 <b>Token:</b> {}\n\
            📍 <b>Address:</b> <code>{}</code>\n\
            💰 <b>Amount:</b> {} SOL\n\
            {} <b>{}:</b> {} SOL ({:.2}%)\n\
            ⏰ <b>Time:</b> {}\n\n\
            🔗 <a href=\"https://dexscreener.com/solana/{}\">View on DexScreener</a>",
            emoji, emoji,
            token_name,
            token_address,
            amount_sol,
            emoji, status, profit_loss, percentage,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            token_address
        );

        self.send_message(&message).await
    }
}