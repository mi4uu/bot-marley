use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub date: String,
    pub time: String,
    pub asset: String,
    pub pair: String,
    pub transaction_type: String, // "BUY" or "SELL"
    pub amount: f64,
    pub price_per_unit: f64,
    pub amount_in_usd: f64,
    pub profit: f64,
    pub profit_in_usd: f64,
    pub profit_percent: f64,
    pub total_asset: f64,
    pub total_asset_worth_usd: f64,
}

#[derive(Debug, Clone)]
pub struct AssetPosition {
    pub total_amount: f64,
    pub average_buy_price: f64,
    pub total_invested_usd: f64,
}

pub struct TransactionTracker {
    transactions_file: String,
    positions: HashMap<String, AssetPosition>,
}

impl TransactionTracker {
    pub fn new(transactions_file: &str) -> Self {
        let mut tracker = Self {
            transactions_file: transactions_file.to_string(),
            positions: HashMap::new(),
        };
        
        // Load existing positions from transaction history
        tracker.load_positions_from_history();
        tracker
    }

    fn load_positions_from_history(&mut self) {
        if !Path::new(&self.transactions_file).exists() {
            return;
        }

        match File::open(&self.transactions_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if let Ok(transaction) = serde_json::from_str::<Transaction>(&line) {
                            self.update_position_from_transaction(&transaction);
                        }
                    }
                }
                info!("📊 Loaded {} asset positions from transaction history", self.positions.len());
            }
            Err(e) => {
                warn!("⚠️ Could not load transaction history: {}", e);
            }
        }
    }

    fn update_position_from_transaction(&mut self, transaction: &Transaction) {
        let asset = &transaction.asset;
        
        match transaction.transaction_type.as_str() {
            "BUY" => {
                let position = self.positions.entry(asset.clone()).or_insert(AssetPosition {
                    total_amount: 0.0,
                    average_buy_price: 0.0,
                    total_invested_usd: 0.0,
                });
                
                // Update average buy price using weighted average
                let new_total_invested = position.total_invested_usd + transaction.amount_in_usd;
                let new_total_amount = position.total_amount + transaction.amount;
                
                if new_total_amount > 0.0 {
                    position.average_buy_price = new_total_invested / new_total_amount;
                }
                
                position.total_amount = new_total_amount;
                position.total_invested_usd = new_total_invested;
            }
            "SELL" => {
                if let Some(position) = self.positions.get_mut(asset) {
                    position.total_amount -= transaction.amount;
                    // Reduce invested amount proportionally
                    let sell_ratio = transaction.amount / (position.total_amount + transaction.amount);
                    position.total_invested_usd -= position.total_invested_usd * sell_ratio;
                    
                    // Remove position if amount is negligible
                    if position.total_amount < 0.0001 {
                        self.positions.remove(asset);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn record_buy_transaction(
        &mut self,
        pair: &str,
        amount: f64,
        price_per_unit: f64,
        current_price_usd: f64,
    ) -> Result<Transaction, Box<dyn std::error::Error>> {
        let asset = self.extract_asset_from_pair(pair);
        let amount_in_usd = amount * price_per_unit;
        
        // For buy transactions, profit is 0 initially
        let transaction = Transaction {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            time: Utc::now().format("%H:%M:%S").to_string(),
            asset: asset.clone(),
            pair: pair.to_string(),
            transaction_type: "BUY".to_string(),
            amount,
            price_per_unit,
            amount_in_usd,
            profit: 0.0,
            profit_in_usd: 0.0,
            profit_percent: 0.0,
            total_asset: 0.0, // Will be updated after position update
            total_asset_worth_usd: 0.0, // Will be updated after position update
        };

        // Update position
        self.update_position_from_transaction(&transaction);
        
        // Update total asset values in transaction
        let mut final_transaction = transaction;
        if let Some(position) = self.positions.get(&asset) {
            final_transaction.total_asset = position.total_amount;
            final_transaction.total_asset_worth_usd = position.total_amount * current_price_usd;
        }

        // Save to file
        self.save_transaction(&final_transaction)?;
        
        info!("💰 BUY transaction recorded: {} {} at ${:.4}", amount, asset, price_per_unit);
        Ok(final_transaction)
    }

    pub fn record_sell_transaction(
        &mut self,
        pair: &str,
        amount: f64,
        price_per_unit: f64,
        current_price_usd: f64,
    ) -> Result<Transaction, Box<dyn std::error::Error>> {
        let asset = self.extract_asset_from_pair(pair);
        let amount_in_usd = amount * price_per_unit;
        
        // Calculate profit
        let (profit, profit_in_usd, profit_percent) = if let Some(position) = self.positions.get(&asset) {
            let cost_basis = position.average_buy_price * amount;
            let profit_native = (price_per_unit - position.average_buy_price) * amount;
            let profit_usd = amount_in_usd - cost_basis;
            let profit_pct = if cost_basis > 0.0 {
                (profit_usd / cost_basis) * 100.0
            } else {
                0.0
            };
            (profit_native, profit_usd, profit_pct)
        } else {
            // No position found, assume break-even
            (0.0, 0.0, 0.0)
        };

        let transaction = Transaction {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            time: Utc::now().format("%H:%M:%S").to_string(),
            asset: asset.clone(),
            pair: pair.to_string(),
            transaction_type: "SELL".to_string(),
            amount,
            price_per_unit,
            amount_in_usd,
            profit,
            profit_in_usd,
            profit_percent,
            total_asset: 0.0, // Will be updated after position update
            total_asset_worth_usd: 0.0, // Will be updated after position update
        };

        // Update position
        self.update_position_from_transaction(&transaction);
        
        // Update total asset values in transaction
        let mut final_transaction = transaction;
        if let Some(position) = self.positions.get(&asset) {
            final_transaction.total_asset = position.total_amount;
            final_transaction.total_asset_worth_usd = position.total_amount * current_price_usd;
        }

        // Save to file
        self.save_transaction(&final_transaction)?;
        
        info!("💸 SELL transaction recorded: {} {} at ${:.4} (Profit: ${:.2})", 
              amount, asset, price_per_unit, profit_in_usd);
        Ok(final_transaction)
    }

    fn extract_asset_from_pair(&self, pair: &str) -> String {
        // Extract base asset from trading pair (e.g., "BTCUSDT" -> "BTC")
        if pair.ends_with("USDT") {
            pair.strip_suffix("USDT").unwrap_or(pair).to_string()
        } else if pair.ends_with("USDC") {
            pair.strip_suffix("USDC").unwrap_or(pair).to_string()
        } else if pair.ends_with("BTC") {
            pair.strip_suffix("BTC").unwrap_or(pair).to_string()
        } else if pair.ends_with("ETH") {
            pair.strip_suffix("ETH").unwrap_or(pair).to_string()
        } else {
            // Fallback: assume first 3-4 characters are the asset
            if pair.len() >= 6 {
                pair[..pair.len()-4].to_string()
            } else {
                pair.to_string()
            }
        }
    }

    fn save_transaction(&self, transaction: &Transaction) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.transactions_file)?;
        
        let json_line = serde_json::to_string(transaction)?;
        writeln!(file, "{}", json_line)?;
        
        Ok(())
    }

    pub fn get_total_portfolio_value_usd(&self, current_prices: &HashMap<String, f64>) -> f64 {
        let mut total_value = 0.0;
        
        for (asset, position) in &self.positions {
            if let Some(&current_price) = current_prices.get(asset) {
                total_value += position.total_amount * current_price;
            }
        }
        
        total_value
    }

    pub fn get_total_profit_usd(&self) -> f64 {
        if !Path::new(&self.transactions_file).exists() {
            return 0.0;
        }

        let mut total_profit = 0.0;
        
        match File::open(&self.transactions_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if let Ok(transaction) = serde_json::from_str::<Transaction>(&line) {
                            if transaction.transaction_type == "SELL" {
                                total_profit += transaction.profit_in_usd;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("❌ Error reading transactions file: {}", e);
            }
        }
        
        total_profit
    }

    pub fn get_position(&self, asset: &str) -> Option<&AssetPosition> {
        self.positions.get(asset)
    }
}