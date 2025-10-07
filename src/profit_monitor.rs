// Real-time Profit Monitoring Module
use crate::settings::BotSettings;
use crate::telegram::TelegramNotifier;
use crate::pool_scanner::TokenPosition;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use anyhow::Result;
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitData {
    pub token_address: String,
    pub symbol: String,
    pub current_price_usd: f64,
    pub entry_price_usd: f64,
    pub current_value_sol: f64,
    pub entry_value_sol: f64,
    pub pnl_sol: f64,
    pub pnl_percentage: f64,
    pub pnl_usd: f64,
    pub tokens_held: u64,
    pub last_updated: SystemTime,
    pub highest_value: f64,
    pub lowest_value: f64,
    pub time_held: Duration,
}

#[derive(Debug, Clone)]
pub struct ProfitAlert {
    pub token_address: String,
    pub alert_type: AlertType,
    pub percentage: f64,
    pub value_sol: f64,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertType {
    ProfitTarget(f64),     // +X% profit reached
    StopLoss(f64),         // -X% loss reached
    TrailingStop(f64),     // Trailing stop triggered
    TimeAlert(Duration),   // Position held for X time
    VolumeSpike,           // Unusual volume detected
    LiquidityChange,       // Liquidity pool changes
}

#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub total_invested_sol: f64,
    pub current_value_sol: f64,
    pub total_pnl_sol: f64,
    pub total_pnl_percentage: f64,
    pub total_pnl_usd: f64,
    pub active_positions: usize,
    pub winning_positions: usize,
    pub losing_positions: usize,
    pub best_performer: Option<String>,
    pub worst_performer: Option<String>,
    pub daily_high: f64,
    pub daily_low: f64,
}

pub struct ProfitMonitor {
    settings: BotSettings,
    telegram: TelegramNotifier,
    profit_data: HashMap<String, ProfitData>,
    price_cache: HashMap<String, (f64, SystemTime)>,
    alert_history: Vec<ProfitAlert>,
    portfolio_history: Vec<PortfolioSummary>,
    client: reqwest::Client,
    sol_price_usd: f64,
    last_sol_price_update: SystemTime,
    last_telegram_update: SystemTime,
    last_portfolio_summary: SystemTime,
    pub is_monitoring: bool,
    last_price_update: SystemTime,
}

impl ProfitMonitor {
    pub fn new(settings: BotSettings, telegram: TelegramNotifier) -> Self {
        Self {
            settings,
            telegram,
            profit_data: HashMap::new(),
            price_cache: HashMap::new(),
            alert_history: Vec::new(),
            portfolio_history: Vec::new(),
            client: reqwest::Client::new(),
            sol_price_usd: 0.0,
            last_sol_price_update: SystemTime::UNIX_EPOCH,
            last_telegram_update: SystemTime::now(),
            last_portfolio_summary: SystemTime::now(),
            is_monitoring: false,
            last_price_update: SystemTime::now(),
        }
    }

    /// Start the real-time monitoring loop
    pub async fn start_monitoring(&mut self, positions: &HashMap<String, TokenPosition>) -> Result<()> {
        // Only initialize if not already monitoring
        if !self.is_monitoring {
            println!("🚀 Starting real-time profit monitor...");
            self.is_monitoring = true;
        }
        
        // Initialize profit data for new positions only
        for (_token_address, position) in positions {
            if !self.profit_data.contains_key(&position.token_address) {
                if let Err(e) = self.add_position(position).await {
                    println!("⚠️  Failed to add position {}: {}", position.token_address, e);
                }
            }
        }

        // Update existing positions periodically
        self.update_monitoring_cycle().await
    }

    /// Update monitoring cycle (called periodically, not in infinite loop)
    async fn update_monitoring_cycle(&mut self) -> Result<()> {
        let now = SystemTime::now();
        
        // Update prices every 30 seconds
        if now.duration_since(self.last_price_update).unwrap_or_default() > Duration::from_secs(30) {
            if let Err(e) = self.update_all_prices().await {
                println!("⚠️ Error updating prices: {}", e);
            }
            self.last_price_update = now;
        }
        
        // Send Telegram updates every 5 minutes
        if now.duration_since(self.last_telegram_update).unwrap_or_default() > Duration::from_secs(5 * 60) {
            if let Err(e) = self.send_telegram_update().await {
                println!("⚠️ Error sending Telegram update: {}", e);
            }
            self.last_telegram_update = now;
        }
        
        // Generate portfolio summary every 15 minutes
        if now.duration_since(self.last_portfolio_summary).unwrap_or_default() > Duration::from_secs(15 * 60) {
            if let Err(e) = self.generate_portfolio_summary().await {
                println!("⚠️ Error generating portfolio summary: {}", e);
            }
            self.last_portfolio_summary = now;
        }
        
        Ok(())
    }

    /// Add a new position to monitor
    pub async fn add_position(&mut self, position: &TokenPosition) -> Result<()> {
        println!("📊 Adding position to profit monitor: {}", position.token_address);

        // Check if position already exists to avoid duplicates
        if self.profit_data.contains_key(&position.token_address) {
            return Ok(());
        }

        // For simulated/test tokens, use simulated price data
        let (current_price_usd, symbol) = if position.token_address.starts_with("Sample-") ||
                                             position.token_address.len() < 32 || // Real Solana addresses are 32+ chars
                                             position.token_address.contains("Sample") {
            // Use entry price as current price for simulation
            let sol_price = self.get_sol_price().await.unwrap_or(150.0); // Fallback SOL price
            let simulated_price = position.entry_price * sol_price;
            (simulated_price, "SIM-TOKEN".to_string())
        } else {
            // Try to get real price data
            match self.get_token_price(&position.token_address).await {
                Ok(price_data) => price_data,
                Err(_) => {
                    // Fallback for tokens without price data
                    let sol_price = self.get_sol_price().await.unwrap_or(150.0);
                    let fallback_price = position.entry_price * sol_price;
                    (fallback_price, "UNKNOWN".to_string())
                }
            }
        };
        
        let sol_price = self.get_sol_price().await.unwrap_or(150.0);
        
        let entry_price_usd = position.entry_price * sol_price;
        let current_value_sol = (position.estimated_tokens as f64 * current_price_usd) / sol_price;
        
        let profit_data = ProfitData {
            token_address: position.token_address.clone(),
            symbol,
            current_price_usd,
            entry_price_usd,
            current_value_sol,
            entry_value_sol: position.sol_amount,
            pnl_sol: current_value_sol - position.sol_amount,
            pnl_percentage: ((current_value_sol - position.sol_amount) / position.sol_amount) * 100.0,
            pnl_usd: (current_value_sol - position.sol_amount) * sol_price,
            tokens_held: position.estimated_tokens,
            last_updated: SystemTime::now(),
            highest_value: current_value_sol.max(position.sol_amount),
            lowest_value: current_value_sol.min(position.sol_amount),
            time_held: SystemTime::now().duration_since(position.purchase_time).unwrap_or_default(),
        };

        self.profit_data.insert(position.token_address.clone(), profit_data);
        Ok(())
    }

    /// Remove a position from monitoring (when sold)
    pub fn remove_position(&mut self, token_address: &str) -> Option<ProfitData> {
        self.profit_data.remove(token_address)
    }

    /// Update prices for all monitored positions
    async fn update_all_prices(&mut self) -> Result<()> {
        let sol_price = self.get_sol_price().await?;
        
        // Collect token addresses to avoid borrow checker issues
        let token_addresses: Vec<String> = self.profit_data.keys().cloned().collect();
        
        for token_address in token_addresses {
            // Get token price first
            match self.get_token_price(&token_address).await {
                Ok((current_price_usd, _)) => {
                    // Now update the profit data
                    if let Some(profit_data) = self.profit_data.get_mut(&token_address) {
                        let current_value_sol = (profit_data.tokens_held as f64 * current_price_usd) / sol_price;
                        
                        // Update profit data
                        profit_data.current_price_usd = current_price_usd;
                        profit_data.current_value_sol = current_value_sol;
                        profit_data.pnl_sol = current_value_sol - profit_data.entry_value_sol;
                        profit_data.pnl_percentage = (profit_data.pnl_sol / profit_data.entry_value_sol) * 100.0;
                        profit_data.pnl_usd = profit_data.pnl_sol * sol_price;
                        profit_data.last_updated = SystemTime::now();
                        
                        // Update high/low tracking
                        profit_data.highest_value = profit_data.highest_value.max(current_value_sol);
                        profit_data.lowest_value = profit_data.lowest_value.min(current_value_sol);
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to update price for {}: {}", token_address, e);
                }
            }
        }
        
        Ok(())
    }

    /// Get current token price from DexScreener
    async fn get_token_price(&mut self, token_address: &str) -> Result<(f64, String)> {
        // Check cache first (cache for 30 seconds)
        if let Some((cached_price, cached_time)) = self.price_cache.get(token_address) {
            if cached_time.elapsed().unwrap_or_default() < Duration::from_secs(30) {
                return Ok((*cached_price, "CACHED".to_string()));
            }
        }

        let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", token_address);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let Some(pairs) = data["pairs"].as_array() {
                            if let Some(pair) = pairs.first() {
                                let price_usd = pair["priceUsd"]
                                    .as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                
                                let symbol = pair["baseToken"]["symbol"]
                                    .as_str()
                                    .unwrap_or("UNKNOWN")
                                    .to_string();
                                
                                // Cache the price
                                self.price_cache.insert(token_address.to_string(), (price_usd, SystemTime::now()));
                                
                                return Ok((price_usd, symbol));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Price fetch error for {}: {}", token_address, e);
            }
        }
        
        // Return cached value if API fails
        if let Some((cached_price, _)) = self.price_cache.get(token_address) {
            return Ok((*cached_price, "STALE".to_string()));
        }
        
        Err(anyhow::anyhow!("Failed to get price for {}", token_address))
    }

    /// Get current SOL price in USD
    async fn get_sol_price(&mut self) -> Result<f64> {
        // Cache SOL price for 5 minutes
        if self.last_sol_price_update.elapsed().unwrap_or_default() < Duration::from_secs(300) && self.sol_price_usd > 0.0 {
            return Ok(self.sol_price_usd);
        }

        let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
        
        match self.client.get(url).send().await {
            Ok(response) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if let Some(sol_price) = data["solana"]["usd"].as_f64() {
                        self.sol_price_usd = sol_price;
                        self.last_sol_price_update = SystemTime::now();
                        return Ok(sol_price);
                    }
                }
            }
            Err(_) => {
                // Fallback: use Jupiter price API
                if let Ok(response) = self.client.get("https://price.jup.ag/v6/price?ids=SOL").send().await {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let Some(sol_price) = data["data"]["SOL"]["price"].as_f64() {
                            self.sol_price_usd = sol_price;
                            self.last_sol_price_update = SystemTime::now();
                            return Ok(sol_price);
                        }
                    }
                }
            }
        }
        
        // Use cached value if available
        if self.sol_price_usd > 0.0 {
            return Ok(self.sol_price_usd);
        }
        
        // Fallback to estimated value
        Ok(150.0) // Rough SOL price fallback
    }

    /// Check for profit/loss alerts and send notifications
    async fn check_profit_alerts(&mut self) -> Result<()> {
        // Collect alerts to send to avoid borrow checker issues
        let mut alerts_to_send = Vec::new();
        
        for (token_address, profit_data) in &self.profit_data {
            // Check profit targets
            if profit_data.pnl_percentage >= self.settings.trading.profit_threshold_percent {
                alerts_to_send.push((token_address.clone(), AlertType::ProfitTarget(profit_data.pnl_percentage), profit_data.clone()));
            }
            
            // Check stop loss
            if profit_data.pnl_percentage <= -self.settings.trading.stop_loss_percent {
                alerts_to_send.push((token_address.clone(), AlertType::StopLoss(profit_data.pnl_percentage), profit_data.clone()));
            }
            
            // Check trailing stop
            let decline_from_high = ((profit_data.highest_value - profit_data.current_value_sol) / profit_data.highest_value) * 100.0;
            if decline_from_high >= self.settings.trading.trailing_stop_percent {
                alerts_to_send.push((token_address.clone(), AlertType::TrailingStop(decline_from_high), profit_data.clone()));
            }
            
            // Check time-based alerts (every 30 minutes)
            if profit_data.time_held.as_secs() % 1800 == 0 && profit_data.time_held.as_secs() > 0 {
                alerts_to_send.push((token_address.clone(), AlertType::TimeAlert(profit_data.time_held), profit_data.clone()));
            }
        }
        
        // Send all collected alerts
        for (token_address, alert_type, profit_data) in alerts_to_send {
            self.send_profit_alert(&token_address, alert_type, &profit_data).await?;
        }
        
        Ok(())
    }

    /// Send profit alert via Telegram
    async fn send_profit_alert(&mut self, token_address: &str, alert_type: AlertType, profit_data: &ProfitData) -> Result<()> {
        if !self.settings.telegram.notifications_enabled {
            return Ok(());
        }

        let emoji = match &alert_type {
            AlertType::ProfitTarget(_) => "🎯",
            AlertType::StopLoss(_) => "🛑",
            AlertType::TrailingStop(_) => "📈",
            AlertType::TimeAlert(_) => "⏰",
            AlertType::VolumeSpike => "📊",
            AlertType::LiquidityChange => "💧",
        };

        let message = match alert_type {
            AlertType::ProfitTarget(pct) => {
                format!("🎯 PROFIT TARGET HIT!\n💎 Token: `{}`\n💰 P&L: {:.4} SOL ({:.2}%)\n💵 USD Value: ${:.2}\n⏰ Held: {}",
                    profit_data.symbol,
                    profit_data.pnl_sol,
                    pct,
                    profit_data.pnl_usd,
                    format_duration(profit_data.time_held)
                )
            }
            AlertType::StopLoss(pct) => {
                format!("🛑 STOP LOSS TRIGGERED!\n💎 Token: `{}`\n💰 P&L: {:.4} SOL ({:.2}%)\n💵 USD Loss: ${:.2}\n⏰ Held: {}",
                    profit_data.symbol,
                    profit_data.pnl_sol,
                    pct,
                    profit_data.pnl_usd,
                    format_duration(profit_data.time_held)
                )
            }
            AlertType::TrailingStop(decline) => {
                format!("📈 TRAILING STOP!\n💎 Token: `{}`\n📉 Decline from high: {:.2}%\n💰 Current P&L: {:.4} SOL ({:.2}%)\n⏰ Held: {}",
                    profit_data.symbol,
                    decline,
                    profit_data.pnl_sol,
                    profit_data.pnl_percentage,
                    format_duration(profit_data.time_held)
                )
            }
            AlertType::TimeAlert(duration) => {
                format!("⏰ TIME UPDATE\n💎 Token: `{}`\n⏰ Held: {}\n💰 Current P&L: {:.4} SOL ({:.2}%)\n💵 USD Value: ${:.2}",
                    profit_data.symbol,
                    format_duration(duration),
                    profit_data.pnl_sol,
                    profit_data.pnl_percentage,
                    profit_data.pnl_usd
                )
            }
            _ => format!("{} Alert for {}", emoji, profit_data.symbol)
        };

        self.telegram.send_message(&message).await?;
        
        // Record the alert
        self.alert_history.push(ProfitAlert {
            token_address: token_address.to_string(),
            alert_type,
            percentage: profit_data.pnl_percentage,
            value_sol: profit_data.current_value_sol,
            timestamp: SystemTime::now(),
        });
        
        Ok(())
    }

    /// Send regular Telegram update with portfolio status
    async fn send_telegram_update(&self) -> Result<()> {
        if !self.settings.telegram.notifications_enabled || self.profit_data.is_empty() {
            return Ok(());
        }

        let summary = self.calculate_portfolio_summary();
        
        let mut message = format!(
            "📊 PORTFOLIO UPDATE\n\n💰 Total Invested: {:.4} SOL\n💎 Current Value: {:.4} SOL\n📈 Total P&L: {:.4} SOL ({:.2}%)\n💵 USD P&L: ${:.2}\n\n🎯 Active Positions: {}\n✅ Winning: {} | ❌ Losing: {}\n\n",
            summary.total_invested_sol,
            summary.current_value_sol,
            summary.total_pnl_sol,
            summary.total_pnl_percentage,
            summary.total_pnl_usd,
            summary.active_positions,
            summary.winning_positions,
            summary.losing_positions
        );

        // Add top 3 performers
        let mut sorted_positions: Vec<_> = self.profit_data.values().collect();
        sorted_positions.sort_by(|a, b| b.pnl_percentage.partial_cmp(&a.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal));

        message.push_str("🏆 TOP PERFORMERS:\n");
        for (i, profit_data) in sorted_positions.iter().take(3).enumerate() {
            let emoji = if profit_data.pnl_percentage > 0.0 { "📈" } else { "📉" };
            message.push_str(&format!(
                "{}. {} `{}`: {:.2}% ({:.4} SOL)\n",
                i + 1,
                emoji,
                profit_data.symbol,
                profit_data.pnl_percentage,
                profit_data.pnl_sol
            ));
        }

        self.telegram.send_message(&message).await?;
        Ok(())
    }

    /// Generate and store portfolio summary
    async fn generate_portfolio_summary(&mut self) -> Result<()> {
        let summary = self.calculate_portfolio_summary();
        self.portfolio_history.push(summary.clone());
        
        // Keep only last 24 hours of history (96 entries at 15-min intervals)
        if self.portfolio_history.len() > 96 {
            self.portfolio_history.remove(0);
        }
        
        println!("📊 Portfolio Summary: {:.4} SOL total value, {:.2}% P&L", 
            summary.current_value_sol, summary.total_pnl_percentage);
        
        Ok(())
    }

    /// Calculate current portfolio summary
    fn calculate_portfolio_summary(&self) -> PortfolioSummary {
        if self.profit_data.is_empty() {
            return PortfolioSummary {
                total_invested_sol: 0.0,
                current_value_sol: 0.0,
                total_pnl_sol: 0.0,
                total_pnl_percentage: 0.0,
                total_pnl_usd: 0.0,
                active_positions: 0,
                winning_positions: 0,
                losing_positions: 0,
                best_performer: None,
                worst_performer: None,
                daily_high: 0.0,
                daily_low: 0.0,
            };
        }

        let total_invested = self.profit_data.values().map(|p| p.entry_value_sol).sum::<f64>();
        let current_value = self.profit_data.values().map(|p| p.current_value_sol).sum::<f64>();
        let total_pnl = current_value - total_invested;
        let total_pnl_percentage = if total_invested > 0.0 { (total_pnl / total_invested) * 100.0 } else { 0.0 };
        let total_pnl_usd = self.profit_data.values().map(|p| p.pnl_usd).sum::<f64>();

        let winning_positions = self.profit_data.values().filter(|p| p.pnl_sol > 0.0).count();
        let losing_positions = self.profit_data.values().filter(|p| p.pnl_sol < 0.0).count();

        let best_performer = self.profit_data.values()
            .max_by(|a, b| a.pnl_percentage.partial_cmp(&b.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal))
            .map(|p| p.symbol.clone());

        let worst_performer = self.profit_data.values()
            .min_by(|a, b| a.pnl_percentage.partial_cmp(&b.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal))
            .map(|p| p.symbol.clone());

        PortfolioSummary {
            total_invested_sol: total_invested,
            current_value_sol: current_value,
            total_pnl_sol: total_pnl,
            total_pnl_percentage,
            total_pnl_usd,
            active_positions: self.profit_data.len(),
            winning_positions,
            losing_positions,
            best_performer,
            worst_performer,
            daily_high: current_value, // Simplified - could track actual daily high/low
            daily_low: current_value,
        }
    }

    /// Update console display with real-time profit information
    async fn update_console_display(&self) {
        if self.profit_data.is_empty() {
            return;
        }

        // Clear screen and show header
        print!("\x1B[2J\x1B[1;1H"); // Clear screen
        println!("╔══════════════════════════════════════════════════════════════════════════════╗");
        println!("║                        🚀 REAL-TIME PROFIT MONITOR 🚀                        ║");
        println!("╚══════════════════════════════════════════════════════════════════════════════╝");
        
        let summary = self.calculate_portfolio_summary();
        
        // Portfolio overview
        println!("\n📊 PORTFOLIO OVERVIEW:");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ 💰 Total Invested: {:.4} SOL                                                  │", summary.total_invested_sol);
        println!("│ 💎 Current Value:  {:.4} SOL                                                  │", summary.current_value_sol);
        println!("│ 📈 Total P&L:      {:.4} SOL ({:.2}%)                                        │", summary.total_pnl_sol, summary.total_pnl_percentage);
        println!("│ 💵 USD P&L:        ${:.2}                                                     │", summary.total_pnl_usd);
        println!("│ 🎯 Positions:      {} active (✅ {} winning, ❌ {} losing)                    │", 
            summary.active_positions, summary.winning_positions, summary.losing_positions);
        println!("└─────────────────────────────────────────────────────────────────────────────┘");

        // Individual positions
        println!("\n💎 ACTIVE POSITIONS:");
        println!("┌─────────────┬──────────┬────────────┬────────────┬──────────┬─────────────┐");
        println!("│   SYMBOL    │   P&L%   │  P&L SOL   │  P&L USD   │   TIME   │   STATUS    │");
        println!("├─────────────┼──────────┼────────────┼────────────┼──────────┼─────────────┤");

        let mut sorted_positions: Vec<_> = self.profit_data.values().collect();
        sorted_positions.sort_by(|a, b| b.pnl_percentage.partial_cmp(&a.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal));

        for profit_data in sorted_positions {
            let status_emoji = if profit_data.pnl_percentage > 10.0 {
                "🚀"
            } else if profit_data.pnl_percentage > 0.0 {
                "📈"
            } else if profit_data.pnl_percentage > -10.0 {
                "📉"
            } else {
                "💀"
            };

            let time_str = format_duration(profit_data.time_held);
            
            println!("│ {:>11} │ {:>7.2}% │ {:>9.4} │ {:>9.2} │ {:>8} │ {:>11} │",
                truncate_string(&profit_data.symbol, 11),
                profit_data.pnl_percentage,
                profit_data.pnl_sol,
                profit_data.pnl_usd,
                time_str,
                status_emoji
            );
        }
        
        println!("└─────────────┴──────────┴────────────┴────────────┴──────────┴─────────────┘");
        
        // Recent alerts
        if !self.alert_history.is_empty() {
            println!("\n🔔 RECENT ALERTS:");
            for alert in self.alert_history.iter().rev().take(3) {
                let time_str = alert.timestamp.elapsed()
                    .map(|d| format!("{}m ago", d.as_secs() / 60))
                    .unwrap_or_else(|_| "now".to_string());
                
                match &alert.alert_type {
                    AlertType::ProfitTarget(pct) => println!("   🎯 Profit target hit: {:.2}% ({})", pct, time_str),
                    AlertType::StopLoss(pct) => println!("   🛑 Stop loss: {:.2}% ({})", pct, time_str),
                    AlertType::TrailingStop(pct) => println!("   📈 Trailing stop: {:.2}% ({})", pct, time_str),
                    AlertType::TimeAlert(_) => println!("   ⏰ Time update ({})", time_str),
                    _ => println!("   🔔 Alert ({})", time_str),
                }
            }
        }
        
        println!("\n⏰ Last updated: {}", chrono::Utc::now().format("%H:%M:%S UTC"));
        println!("💡 Monitoring {} positions in real-time...\n", self.profit_data.len());
    }

    /// Get profit data for a specific token
    pub fn get_profit_data(&self, token_address: &str) -> Option<&ProfitData> {
        self.profit_data.get(token_address)
    }

    /// Get portfolio summary
    pub fn get_portfolio_summary(&self) -> PortfolioSummary {
        self.calculate_portfolio_summary()
    }

    /// Get all profit data
    pub fn get_all_profit_data(&self) -> &HashMap<String, ProfitData> {
        &self.profit_data
    }
}

// Helper functions
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    
    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}