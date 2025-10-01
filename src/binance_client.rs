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