use mono_ai_macros::tool;
use tracing::info;
use crate::binance_client::BinanceClient;
use crate::config::Config;

// Note: Removed async tools due to runtime conflicts with Binance client
// Account information is now provided at bot startup instead

#[tool]
/// Sell asset, confidence in % about this decision , THIS IS FINAL DECISION
fn sell(pair: String, amount:f64, confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: SELL 💰");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION : {} ", explanation);
    format!("Sold {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn buy(pair: String, amount:f64,confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: BUY 🛍️");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION : {} ", explanation);
    format!("Buy {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn hold(pair: String,confidence: usize, explanation: String) -> String {
    info!("FINAL DECISION: HOLD ⌛");
    info!("CONFIDENCE: {} %", confidence);
    info!("EXPLANATION : {} ", explanation);

    format!("Hold for pair {}",  pair)
}
