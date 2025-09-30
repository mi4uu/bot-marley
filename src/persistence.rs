use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDecision {
    pub symbol: String,
    pub action: String, // "BUY", "SELL", "HOLD"
    pub amount: Option<f64>,
    pub confidence: usize,
    pub explanation: String,
    pub timestamp: DateTime<Utc>,
    pub price_at_decision: Option<f64>,
    pub price_timestamp: Option<u64>, // Binance close_time timestamp for deduplication
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHistory {
    pub symbol: String,
    pub decisions: Vec<TradingDecision>,
    pub last_decision: Option<TradingDecision>,
    pub total_decisions: usize,
    pub buy_count: usize,
    pub sell_count: usize,
    pub hold_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradingState {
    pub symbols: HashMap<String, SymbolHistory>,
    pub last_updated: Option<DateTime<Utc>>,
    pub total_runs: usize,
}

impl TradingState {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            last_updated: Some(Utc::now()),
            total_runs: 0,
        }
    }

    pub fn add_decision(&mut self, decision: TradingDecision) {
        let symbol = decision.symbol.clone();
        
        let history = self.symbols.entry(symbol.clone()).or_insert_with(|| SymbolHistory {
            symbol: symbol.clone(),
            decisions: Vec::new(),
            last_decision: None,
            total_decisions: 0,
            buy_count: 0,
            sell_count: 0,
            hold_count: 0,
        });

        // Update counters
        match decision.action.as_str() {
            "BUY" => history.buy_count += 1,
            "SELL" => history.sell_count += 1,
            "HOLD" => history.hold_count += 1,
            _ => {}
        }

        history.decisions.push(decision.clone());
        history.last_decision = Some(decision);
        history.total_decisions += 1;
        
        self.last_updated = Some(Utc::now());
    }

    pub fn get_symbol_history(&self, symbol: &str) -> Option<&SymbolHistory> {
        self.symbols.get(symbol)
    }

    pub fn get_last_decision(&self, symbol: &str) -> Option<&TradingDecision> {
        self.symbols.get(symbol)?.last_decision.as_ref()
    }

    pub fn increment_runs(&mut self) {
        self.total_runs += 1;
        self.last_updated = Some(Utc::now());
    }

    /// Check if a decision already exists for the same symbol and price timestamp
    pub fn has_decision_for_timestamp(&self, symbol: &str, price_timestamp: u64) -> bool {
        if let Some(history) = self.symbols.get(symbol) {
            history.decisions.iter().any(|decision| {
                decision.price_timestamp == Some(price_timestamp)
            })
        } else {
            false
        }
    }

    /// Get the latest price timestamp for a symbol (if any decisions exist)
    pub fn get_latest_price_timestamp(&self, symbol: &str) -> Option<u64> {
        self.symbols.get(symbol)?
            .decisions
            .iter()
            .filter_map(|d| d.price_timestamp)
            .max()
    }

    pub fn generate_context_summary(&self, symbol: &str) -> String {
        if let Some(history) = self.get_symbol_history(symbol) {
            let mut summary = format!(
                "\n📊 TRADING HISTORY FOR {}:\n", symbol
            );
            
            summary.push_str(&format!(
                "  • Total decisions: {} (Buy: {}, Sell: {}, Hold: {})\n",
                history.total_decisions, history.buy_count, history.sell_count, history.hold_count
            ));

            if let Some(last_decision) = &history.last_decision {
                summary.push_str(&format!(
                    "  • Last decision: {} (Confidence: {}%) at {}\n",
                    last_decision.action,
                    last_decision.confidence,
                    last_decision.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                ));
                summary.push_str(&format!(
                    "  • Last explanation: {}\n",
                    last_decision.explanation
                ));
                if let Some(price) = last_decision.price_at_decision {
                    summary.push_str(&format!(
                        "  • Price at last decision: ${:.4}\n",
                        price
                    ));
                }
            }

            // Show recent decisions (last 3)
            if history.decisions.len() > 1 {
                summary.push_str("  • Recent decisions:\n");
                for decision in history.decisions.iter().rev().take(3) {
                    summary.push_str(&format!(
                        "    - {} ({}%) on {}\n",
                        decision.action,
                        decision.confidence,
                        decision.timestamp.format("%m-%d %H:%M")
                    ));
                }
            }
            
            summary
        } else {
            format!("\n📊 TRADING HISTORY FOR {}:\n  • No previous decisions found\n", symbol)
        }
    }
}

pub struct PersistenceManager {
    file_path: String,
}

impl PersistenceManager {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
        }
    }

    pub fn load_state(&self) -> TradingState {
        if !Path::new(&self.file_path).exists() {
            info!("📁 No existing trading state file found, creating new state");
            return TradingState::new();
        }

        match fs::read_to_string(&self.file_path) {
            Ok(content) => {
                match serde_json::from_str::<TradingState>(&content) {
                    Ok(state) => {
                        info!("✅ Loaded trading state with {} symbols", state.symbols.len());
                        state
                    }
                    Err(e) => {
                        error!("❌ Failed to parse trading state file: {}", e);
                        warn!("🔄 Creating new trading state");
                        TradingState::new()
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to read trading state file: {}", e);
                warn!("🔄 Creating new trading state");
                TradingState::new()
            }
        }
    }

    pub fn save_state(&self, state: &TradingState) -> Result<(), Box<dyn std::error::Error>> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&self.file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(state)?;
        fs::write(&self.file_path, json)?;
        
        info!("💾 Trading state saved to {}", self.file_path);
        Ok(())
    }

    pub fn backup_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(&self.file_path).exists() {
            let backup_path = format!("{}.backup.{}", 
                self.file_path, 
                Utc::now().format("%Y%m%d_%H%M%S")
            );
            fs::copy(&self.file_path, &backup_path)?;
            info!("🔄 Created backup at {}", backup_path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_state_operations() {
        let mut state = TradingState::new();
        
        let decision = TradingDecision {
            symbol: "BTCUSDT".to_string(),
            action: "BUY".to_string(),
            amount: Some(100.0),
            confidence: 85,
            explanation: "Strong bullish signal".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(45000.0),
            price_timestamp: Some(1640995499999),
        };

        state.add_decision(decision);
        
        assert_eq!(state.symbols.len(), 1);
        assert!(state.get_symbol_history("BTCUSDT").is_some());
        assert_eq!(state.get_symbol_history("BTCUSDT").unwrap().buy_count, 1);
    }
}