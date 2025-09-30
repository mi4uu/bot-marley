use mono_ai_macros::tool;
use tracing::{info, warn, error};
use crate::binance_client::BinanceClient;
use crate::config::Config;
use crate::transaction_tracker::TransactionTracker;
use binance::api::*;
use binance::account::*;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::sync::Mutex;
use std::sync::OnceLock;

// Global transaction tracker
static TRANSACTION_TRACKER: OnceLock<Mutex<TransactionTracker>> = OnceLock::new();

fn get_transaction_tracker() -> &'static Mutex<TransactionTracker> {
    TRANSACTION_TRACKER.get_or_init(|| {
        Mutex::new(TransactionTracker::new("data/transactions.jsonl"))
    })
}

// Helper function to get current price for USD value calculation using HTTP API
async fn get_current_price_async(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", symbol);
    
    let response = client
        .get(&url)
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    let price_str = response["price"].as_str()
        .ok_or("Price not found in response")?;
    let price: f64 = price_str.parse()?;
    
    Ok(price)
}

// Sync wrapper for price fetching
fn get_current_price(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(get_current_price_async(symbol))
}

// Helper function to validate trade restrictions
fn validate_trade_restrictions(
    pair: &str,
    amount: f64,
    is_buy: bool,
    config: &Config,
    binance_client: &BinanceClient,
) -> Result<(), String> {
    // Get current price to calculate USD value
    let current_price = get_current_price(pair)
        .map_err(|e| format!("Cannot validate trade - failed to get price: {}", e))?;
    
    let usd_value = amount * current_price;
    
    // Check amount restrictions
    let max_amount = if is_buy {
        config.max_trade_value as f64
    } else {
        config.max_trade_value as f64 * 1.1
    };
    
    if usd_value > max_amount {
        return Err(format!(
            "Trade amount ${:.2} exceeds maximum allowed ${:.2} for {} operation",
            usd_value, max_amount, if is_buy { "BUY" } else { "SELL" }
        ));
    }
    
    // Check max active orders
    let open_orders = binance_client.get_open_orders()
        .map_err(|e| format!("Cannot validate active orders: {}", e))?;
    
    if open_orders.len() >= config.max_active_orders {
        return Err(format!(
            "Maximum active orders ({}) reached. Current open orders: {}",
            config.max_active_orders, open_orders.len()
        ));
    }
    
    Ok(())
}

// Helper function to create Binance client
fn create_binance_client() -> Result<BinanceClient, String> {
    let api_key = env::var("BINANCE_API_KEY")
        .map_err(|_| "BINANCE_API_KEY environment variable not set")?;
    let secret_key = env::var("BINANCE_SECRET_KEY")
        .map_err(|_| "BINANCE_SECRET_KEY environment variable not set")?;
    
    if api_key == "noop" || secret_key == "noop" {
        return Err("Binance API credentials not configured (set to 'noop')".to_string());
    }
    
    BinanceClient::new(api_key, secret_key)
        .map_err(|e| format!("Failed to create Binance client: {}", e))
}

#[tool]
/// Sell asset, confidence in % about this decision, THIS IS FINAL DECISION
fn sell(pair: String, amount: f64, confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: SELL 💰");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION: {}", explanation);
    
    // Load config
    let config = Config::load();
    
    // Create Binance client
    let binance_client = match create_binance_client() {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("❌ Cannot execute SELL: {}", e);
            warn!("{}", error_msg);
            return error_msg;
        }
    };
    
    // Validate trade restrictions
    if let Err(e) = validate_trade_restrictions(&pair, amount, false, &config, &binance_client) {
        let error_msg = format!("❌ SELL validation failed: {}", e);
        warn!("{}", error_msg);
        return error_msg;
    }
    
    // Execute the sell order
    match execute_sell_order(&pair, amount, &binance_client) {
        Ok(result) => {
            let success_msg = format!("✅ SELL executed: {}", result);
            info!("{}", success_msg);
            success_msg
        }
        Err(e) => {
            let error_msg = format!("❌ SELL execution failed: {}", e);
            error!("{}", error_msg);
            error_msg
        }
    }
}

#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn buy(pair: String, amount: f64, confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: BUY 🛍️");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION: {}", explanation);
    
    // Load config
    let config = Config::load();
    
    // Create Binance client
    let binance_client = match create_binance_client() {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("❌ Cannot execute BUY: {}", e);
            warn!("{}", error_msg);
            return error_msg;
        }
    };
    
    // Validate trade restrictions
    if let Err(e) = validate_trade_restrictions(&pair, amount, true, &config, &binance_client) {
        let error_msg = format!("❌ BUY validation failed: {}", e);
        warn!("{}", error_msg);
        return error_msg;
    }
    
    // Execute the buy order
    match execute_buy_order(&pair, amount, &binance_client) {
        Ok(result) => {
            let success_msg = format!("✅ BUY executed: {}", result);
            info!("{}", success_msg);
            success_msg
        }
        Err(e) => {
            let error_msg = format!("❌ BUY execution failed: {}", e);
            error!("{}", error_msg);
            error_msg
        }
    }
}

#[tool]
/// Hold asset, confidence in % about this decision, THIS IS FINAL DECISION
fn hold(pair: String, confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: HOLD ⌛");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION: {}", explanation);
    
    format!("✅ HOLD decision recorded for pair {}", pair)
}

// Helper function to execute buy order
fn execute_buy_order(pair: &str, amount: f64, _binance_client: &BinanceClient) -> Result<String, Box<dyn std::error::Error>> {
    // Get current price for transaction recording
    let current_price = get_current_price(pair)?;
    
    // Create a new account instance for trading
    let api_key = env::var("BINANCE_API_KEY")?;
    let secret_key = env::var("BINANCE_SECRET_KEY")?;
    let account: Account = Binance::new(Some(api_key), Some(secret_key));
    
    // Create market buy order using the correct API
    match account.market_buy(pair, amount) {
        Ok(binance_transaction) => {
            // Record transaction in our tracking system
            let tracker = get_transaction_tracker();
            if let Ok(mut tracker_guard) = tracker.lock() {
                match tracker_guard.record_buy_transaction(pair, amount, current_price, current_price) {
                    Ok(transaction) => {
                        info!("📊 Transaction recorded: BUY {} {} at ${:.4} (Total: ${:.2})",
                              transaction.amount, transaction.asset, transaction.price_per_unit, transaction.amount_in_usd);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to record transaction: {}", e);
                    }
                }
            }
            
            Ok(format!(
                "Market BUY order placed - Symbol: {}, Quantity: {}, Order ID: {}, Status: {}, Price: ${:.4}",
                pair, amount, binance_transaction.order_id, binance_transaction.status, current_price
            ))
        }
        Err(e) => Err(format!("Failed to place BUY order: {}", e).into())
    }
}

// Helper function to execute sell order
fn execute_sell_order(pair: &str, amount: f64, _binance_client: &BinanceClient) -> Result<String, Box<dyn std::error::Error>> {
    // Get current price for transaction recording
    let current_price = get_current_price(pair)?;
    
    // Create a new account instance for trading
    let api_key = env::var("BINANCE_API_KEY")?;
    let secret_key = env::var("BINANCE_SECRET_KEY")?;
    let account: Account = Binance::new(Some(api_key), Some(secret_key));
    
    // Create market sell order using the correct API
    match account.market_sell(pair, amount) {
        Ok(binance_transaction) => {
            // Record transaction in our tracking system
            let tracker = get_transaction_tracker();
            if let Ok(mut tracker_guard) = tracker.lock() {
                match tracker_guard.record_sell_transaction(pair, amount, current_price, current_price) {
                    Ok(transaction) => {
                        info!("📊 Transaction recorded: SELL {} {} at ${:.4} (Profit: ${:.2}, {:.1}%)",
                              transaction.amount, transaction.asset, transaction.price_per_unit,
                              transaction.profit_in_usd, transaction.profit_percent);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to record transaction: {}", e);
                    }
                }
            }
            
            Ok(format!(
                "Market SELL order placed - Symbol: {}, Quantity: {}, Order ID: {}, Status: {}, Price: ${:.4}",
                pair, amount, binance_transaction.order_id, binance_transaction.status, current_price
            ))
        }
        Err(e) => Err(format!("Failed to place SELL order: {}", e).into())
    }
}
