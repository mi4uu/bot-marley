use binance::api::*;
use binance::account::*;
use binance::model::*;
use serde::{Deserialize, Serialize};
use color_eyre::eyre::{Result, WrapErr, eyre};
use color_eyre::Section;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAsset {
    pub asset: String,
    pub free: String,
    pub locked: String,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrder {
    pub symbol: String,
    pub order_id: u64,
    pub side: String,
    pub order_type: String,
    pub quantity: String,
    pub price: String,
    pub status: String,
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTransaction {
    pub symbol: String,
    pub side: String, // BUY or SELL
    pub quantity: String,
    pub price: String,
    pub commission: String,
    pub commission_asset: String,
    pub time: u64,
    pub is_buyer: bool,
}

pub struct BinanceClient {
    account: Account,
}

impl BinanceClient {
    pub fn new(api_key: String, secret_key: String) -> Result<Self> {
        let account = Binance::new(Some(api_key), Some(secret_key));
        Ok(Self { account })
    }

    pub fn get_account_info(&self) -> Result<AccountInformation> {
        self.account.get_account()
            .map_err(|e| eyre!("Failed to get account info: {}", e))
            .with_suggestion(|| "Check your Binance API credentials and permissions")
    }

    pub fn get_user_assets(&self) -> Result<Vec<UserAsset>> {
        let account_info = self.get_account_info()
            .wrap_err("Failed to retrieve account information for assets")?;
        
        let mut assets = Vec::new();
        for balance in account_info.balances {
            let free: f64 = balance.free.parse().unwrap_or(0.0);
            let locked: f64 = balance.locked.parse().unwrap_or(0.0);
            let total = free + locked;
            
            // Only include assets with non-zero balance
            if total > 0.0 {
                assets.push(UserAsset {
                    asset: balance.asset,
                    free: balance.free,
                    locked: balance.locked,
                    total,
                });
            }
        }
        
        // Sort by total balance descending
        assets.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(assets)
    }

    pub fn get_open_orders(&self) -> Result<Vec<UserOrder>> {
        let orders = self.account.get_all_open_orders()
            .map_err(|e| eyre!("Failed to get open orders: {}", e))
            .with_suggestion(|| "Check your Binance API credentials and network connection")?;
            
        let user_orders = orders
            .into_iter()
            .map(|order| UserOrder {
                symbol: order.symbol,
                order_id: order.order_id,
                side: order.side,
                order_type: order.type_name,
                quantity: order.orig_qty,
                price: order.price.to_string(),
                status: order.status,
                time: order.time,
            })
            .collect();
        Ok(user_orders)
    }

    pub fn get_recent_orders(&self, _symbol: Option<String>, _limit: Option<u16>) -> Result<Vec<UserOrder>> {
        // For now, just return open orders as the binance crate API is different
        self.get_open_orders()
    }

    pub fn get_recent_transactions(&self, limit: Option<u16>) -> Result<Vec<RecentTransaction>> {
        let limit = limit.unwrap_or(10);
        
        // Read from local transaction file instead of API for now
        // This provides more reliable access to transaction history
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use std::path::Path;
        
        let transactions_file = "data/transactions.jsonl";
        let mut recent_transactions = Vec::new();
        
        if !Path::new(transactions_file).exists() {
            return Ok(recent_transactions);
        }
        
        match File::open(transactions_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let mut all_transactions = Vec::new();
                
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(transaction) = serde_json::from_str::<crate::transaction_tracker::Transaction>(&line) {
                            // Convert Transaction to RecentTransaction
                            let timestamp = chrono::NaiveDateTime::parse_from_str(
                                &format!("{} {}", transaction.date, transaction.time),
                                "%Y-%m-%d %H:%M:%S"
                            ).unwrap_or_default().and_utc().timestamp() as u64 * 1000;
                            
                            let is_buyer = transaction.transaction_type == "BUY";
                            all_transactions.push(RecentTransaction {
                                symbol: transaction.pair,
                                side: transaction.transaction_type,
                                quantity: transaction.amount.to_string(),
                                price: transaction.price_per_unit.to_string(),
                                commission: "0".to_string(),
                                commission_asset: "".to_string(),
                                time: timestamp,
                                is_buyer,
                            });
                        }
                    }
                }
                
                // Sort by time descending (most recent first)
                all_transactions.sort_by(|a, b| b.time.cmp(&a.time));
                
                // Take only the requested limit
                recent_transactions = all_transactions.into_iter().take(limit as usize).collect();
            }
            Err(e) => {
                return Err(eyre!("Failed to read transactions file: {}", e));
            }
        }
        
        Ok(recent_transactions)
    }

    pub fn format_recent_transactions(&self, limit: Option<u16>) -> Result<String> {
        let transactions = self.get_recent_transactions(limit)
            .wrap_err("Failed to get recent transactions for formatting")?;
        
        if transactions.is_empty() {
            return Ok("📋 **RECENT TRANSACTIONS:** No recent transactions found.\n".to_string());
        }
        
        let mut summary = String::new();
        summary.push_str("📋 **RECENT TRANSACTIONS:**\n");
        
        for (i, tx) in transactions.iter().enumerate() {
            let datetime = chrono::DateTime::from_timestamp(tx.time as i64 / 1000, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M:%S");
            
            let side_emoji = if tx.is_buyer { "🟢" } else { "🔴" };
            
            summary.push_str(&format!(
                "{}. {} {} {} {} at {} ({})\n",
                i + 1,
                side_emoji,
                tx.side,
                tx.quantity,
                tx.symbol,
                tx.price,
                datetime
            ));
        }
        
        summary.push_str("\nUse this transaction history to understand recent trading patterns and make informed decisions.\n");
        
        Ok(summary)
    }

    pub fn format_account_summary(&self) -> Result<String> {
        let assets = self.get_user_assets()
            .wrap_err("Failed to get user assets for summary")?;
        let open_orders = self.get_open_orders()
            .wrap_err("Failed to get open orders for summary")?;
        
        let mut summary = String::new();
        summary.push_str("📊 **ACCOUNT SUMMARY**\n\n");
        
        // Assets section
        summary.push_str("💰 **Current Assets:**\n");
        if assets.is_empty() {
            summary.push_str("- No assets with balance > 0\n");
        } else {
            for asset in assets.iter().take(10) { // Show top 10 assets
                summary.push_str(&format!(
                    "- {}: {} (Free: {}, Locked: {})\n",
                    asset.asset, asset.total, asset.free, asset.locked
                ));
            }
            if assets.len() > 10 {
                summary.push_str(&format!("- ... and {} more assets\n", assets.len() - 10));
            }
        }
        
        summary.push_str("\n");
        
        // Open orders section
        summary.push_str("📋 **Open Orders:**\n");
        if open_orders.is_empty() {
            summary.push_str("- No open orders\n");
        } else {
            for order in open_orders.iter().take(5) { // Show up to 5 open orders
                summary.push_str(&format!(
                    "- {} {}: {} {} @ {} (Status: {})\n",
                    order.symbol, order.side, order.quantity, order.symbol, order.price, order.status
                ));
            }
            if open_orders.len() > 5 {
                summary.push_str(&format!("- ... and {} more open orders\n", open_orders.len() - 5));
            }
        }
        
        summary.push_str("\n");
        summary.push_str("Use this information to make informed trading decisions.\n");
        
        Ok(summary)
    }
}