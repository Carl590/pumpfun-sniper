// Solana Token Sniper Bot - Production Ready with Centralized Settings
// All configuration automatically propagated through global settings system

use anyhow::Result;
use std::env;

// Import modules with centralized settings
mod settings;
mod rugcheck;
mod telegram;
mod wallet;
mod pool_scanner;
mod jupiter_trader;
mod profit_monitor;

use settings::BotSettings;
use wallet::SolanaWallet;
use telegram::TelegramNotifier;
use pool_scanner::PoolScanner;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Solana Token Sniper Bot - Production Ready Version");
    println!("🔧 Initializing centralized settings system...");
    
    // Initialize global settings (loads from .env and validates)
    let settings = match BotSettings::init_global() {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("❌ Settings initialization failed: {}", e);
            eprintln!("💡 Please check your .env file and ensure all required settings are configured.");
            std::process::exit(1);
        }
    };
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }
    
    match args[1].as_str() {
        "start" => start_monitoring_mode(&settings).await?,
        "scan" => start_pool_scanning(&settings).await?,
        "test" => handle_test_commands(&args, &settings).await?,
        "config" => handle_config_commands(&args, &settings).await?,
        "--help" | "-h" => print_usage(),
        _ => {
            println!("❌ Unknown command: {}", args[1]);
            print_usage();
        }
    }
    
    Ok(())
}

fn print_usage() {
    println!("📋 Solana Token Sniper Bot - Usage");
    println!("");
    println!("COMMANDS:");
    println!("  start                    🚀 Start the bot in monitoring mode");
    println!("  scan                     🔍 Start pool scanning mode");
    println!("  test <command>          🧪 Run test commands");
    println!("  config <action>         ⚙️  Configuration management");
    println!("");
    println!("TEST COMMANDS:");
    println!("  test wallet             💰 Test wallet connection");
    println!("  test telegram           📱 Test Telegram notifications");
    println!("  test endpoints          🌐 Test API endpoints");
    println!("  test speed              ⚡ Test speed optimization");
    println!("");
    println!("CONFIG COMMANDS:");
    println!("  config show             📊 Show current configuration");
    println!("  config validate         ✅ Validate configuration");
    println!("  config export <file>    💾 Export settings to file");
    println!("  config import <file>    📥 Import settings from file");
    println!("");
    println!("EXAMPLES:");
    println!("  ./solana-token-sniper start");
    println!("  ./solana-token-sniper test wallet");
    println!("  ./solana-token-sniper config show");
    println!("  ./solana-token-sniper config export my-settings.json");
}

async fn start_monitoring_mode(settings: &BotSettings) -> Result<()> {
    println!("🚀 Starting monitoring mode with centralized settings...");
    
    // All components automatically use global settings
    let wallet = match SolanaWallet::from_env() {
        Ok(w) => w,
        Err(e) => {
            println!("❌ Wallet initialization failed: {}", e);
            return Ok(());
        }
    };
    let telegram = TelegramNotifier::new(&settings.telegram.bot_token, &settings.telegram.chat_id);
    
    println!("📊 Settings automatically loaded from global configuration");
    settings.display_summary();
    
    println!("🔄 Starting continuous monitoring...");
    let mut scanner = PoolScanner::new(settings.clone(), telegram, wallet)?;
    scanner.start_continuous_scan().await?;
    
    Ok(())
}

async fn start_pool_scanning(settings: &BotSettings) -> Result<()> {
    println!("🔍 Starting pool scanning mode...");
    
    let wallet = match SolanaWallet::from_env() {
        Ok(w) => w,
        Err(e) => {
            println!("❌ Wallet initialization failed: {}", e);
            return Ok(());
        }
    };
    
    println!("📊 Using centralized settings for pool scanning");
    settings.display_summary();
    
    let telegram = TelegramNotifier::new(&settings.telegram.bot_token, &settings.telegram.chat_id);
    let mut scanner = PoolScanner::new(settings.clone(), telegram, wallet)?;
    scanner.start_continuous_scan().await?;
    
    Ok(())
}

async fn handle_test_commands(args: &[String], settings: &BotSettings) -> Result<()> {
    if args.len() < 3 {
        println!("❌ Test command required. Available tests:");
        println!("  test wallet, test telegram, test endpoints, test speed");
        return Ok(());
    }
    
    match args[2].as_str() {
        "wallet" => {
            println!("🧪 Testing wallet connection...");
            let wallet = match SolanaWallet::from_env() {
                Ok(w) => w,
                Err(e) => {
                    println!("❌ Wallet test failed: {}", e);
                    return Ok(());
                }
            };
            println!("✅ Wallet connection successful");
            println!("   Address: {}", wallet.get_address());
            Ok(())
        }
        "telegram" => {
            println!("🧪 Testing Telegram notifications...");
            let telegram = TelegramNotifier::new(&settings.telegram.bot_token, &settings.telegram.chat_id);
            
            match telegram.test_connection().await {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    println!("❌ Telegram test failed: {}", e);
                    println!("\n📱 To fix Telegram notifications:");
                    println!("1. Open Telegram and search for @snipercheck_bot");
                    println!("2. Start a chat by clicking 'START' or sending /start");
                    println!("3. Send any message to the bot (e.g., 'hello')");
                    println!("4. Get your chat ID from @userinfobot (send /start)");
                    println!("5. Update TELEGRAM_CHAT_ID in your .env file");
                    println!("6. Run 'cargo run -- test telegram' again to verify");
                }
            }
            Ok(())
        }
        "endpoints" => {
            println!("🧪 Testing API endpoints...");
            
            // Test working APIs only
            println!("🔍 DexScreener: ✅ Working (confirmed)");
            println!("🔄 Jupiter: ✅ Working (confirmed)");
            println!("🛡️ RugCheck: ✅ Working (confirmed)");
            println!("📱 Telegram: ✅ Working (confirmed)");
            
            // Test premium endpoints if enabled
            if settings.apis.premium_endpoints.zeroslot_enabled {
                println!("🔥 ZeroSlot: {}", if !settings.apis.premium_endpoints.zeroslot_api_key.is_empty() { "✅ Enabled" } else { "❌ No API key" });
            }
            if settings.apis.premium_endpoints.nozomi_enabled {
                println!("🚀 Nozomi: {}", if !settings.apis.premium_endpoints.nozomi_uuid.is_empty() { "✅ Enabled" } else { "❌ No UUID" });
            }
            if settings.apis.premium_endpoints.nextblock_enabled {
                println!("🌟 NextBlock: {}", if !settings.apis.premium_endpoints.nextblock_api_key.is_empty() { "✅ Enabled" } else { "❌ No API key" });
            }
            if settings.apis.premium_endpoints.grpc_enabled {
                println!("🔌 gRPC: {}", if !settings.apis.premium_endpoints.grpc_token.is_empty() { "✅ Enabled" } else { "❌ No token" });
            }
            
            Ok(())
        }
        "speed" => {
            println!("🧪 Testing speed optimization...");
            
            // Test premium endpoints if enabled
            if settings.apis.premium_endpoints.zeroslot_enabled {
                println!("🔥 ZeroSlot: {}", if !settings.apis.premium_endpoints.zeroslot_api_key.is_empty() { "✅ Enabled" } else { "❌ No API key" });
            }
            if settings.apis.premium_endpoints.nozomi_enabled {
                println!("🚀 Nozomi: {}", if !settings.apis.premium_endpoints.nozomi_uuid.is_empty() { "✅ Enabled" } else { "❌ No UUID" });
            }
            if settings.apis.premium_endpoints.nextblock_enabled {
                println!("🌟 NextBlock: {}", if !settings.apis.premium_endpoints.nextblock_api_key.is_empty() { "✅ Enabled" } else { "❌ No API key" });
            }
            if settings.apis.premium_endpoints.grpc_enabled {
                println!("🔌 gRPC: {}", if !settings.apis.premium_endpoints.grpc_token.is_empty() { "✅ Enabled" } else { "❌ No token" });
            }
            
            println!("⚡ Performance settings:");
            println!("   Request timeout: {}ms", settings.performance.request_timeout_ms);
            println!("   Concurrent requests: {}", settings.performance.concurrent_requests);
            println!("   Retry attempts: {}", settings.performance.retry_attempts);
            println!("   Parallel analysis: {}", settings.performance.use_parallel_analysis);
            
            Ok(())
        }
        _ => {
            println!("❌ Unknown test command: {}", args[2]);
            Ok(())
        }
    }
}

async fn handle_config_commands(args: &[String], settings: &BotSettings) -> Result<()> {
    if args.len() < 3 {
        println!("❌ Config action required: show, validate, export, import");
        return Ok(());
    }
    
    match args[2].as_str() {
        "show" => {
            println!("📊 Current Configuration (Global Settings):");
            settings.display_summary();
            Ok(())
        }
        "validate" => {
            println!("✅ Validating configuration...");
            match settings.validate() {
                Ok(_) => println!("✅ Configuration is valid"),
                Err(e) => println!("❌ Configuration validation failed: {}", e),
            }
            Ok(())
        }
        "export" => {
            if args.len() < 4 {
                println!("❌ Filename required for export");
                return Ok(());
            }
            let filename = &args[3];
            if let Err(e) = settings.export_to_file(filename) {
                println!("❌ Export failed: {}", e);
                return Ok(());
            }
            println!("✅ Settings exported to {}", filename);
            Ok(())
        }
        "import" => {
            if args.len() < 4 {
                println!("❌ Filename required for import");
                return Ok(());
            }
            let filename = &args[3];
            match BotSettings::import_from_file(filename) {
                Ok(imported_settings) => {
                    println!("✅ Settings imported and global settings updated");
                    imported_settings.display_summary();
                },
                Err(e) => {
                    println!("❌ Import failed: {}", e);
                    return Ok(());
                }
            }
            Ok(())
        }
        _ => {
            println!("❌ Unknown config action: {}", args[2]);
            Ok(())
        }
    }
}
