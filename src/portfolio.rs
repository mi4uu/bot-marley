use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, WrapErr};
use tracing::{info, error};

use crate::binance_client::{BinanceClient, UserAsset};
use crate::tools::get_prices::fetch_klines_cached;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetValue {
    pub asset: String,
    pub balance: f64,
    pub price_usdc: f64,
    pub value_usdc: f64,
    pub price_btc: f64,
    pub value_btc: f64,
    pub percentage_of_portfolio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub timestamp: DateTime<Utc>,
    pub run_number: u32,
    pub btc_price_usdc: f64,
    pub total_value_usdc: f64,
    pub total_value_btc: f64,
    pub assets: Vec<AssetValue>,
    pub asset_count: usize,
}

pub struct PortfolioTracker {
    binance_client: BinanceClient,
}

impl PortfolioTracker {
    pub fn new(binance_client: BinanceClient) -> Self {
        Self { binance_client }
    }

    pub async fn capture_portfolio_snapshot(&self, run_number: u32) -> Result<PortfolioSnapshot> {
        info!("📊 Capturing portfolio snapshot for run #{}", run_number);

        // Get user assets from Binance
        let user_assets = self.binance_client.get_user_assets()
            .wrap_err("Failed to get user assets from Binance")?;

        // Get BTC price in USDC
        let btc_price_usdc = self.get_asset_price_in_usdc("BTC").await
            .unwrap_or_else(|e| {
                error!("Failed to get BTC price: {}", e);
                0.0
            });

        let mut asset_values = Vec::new();
        let mut total_value_usdc = 0.0;

        // Process each asset
        for asset in &user_assets {
            let asset_value = self.calculate_asset_value(&asset, btc_price_usdc).await?;
            total_value_usdc += asset_value.value_usdc;
            asset_values.push(asset_value);
        }

        // Calculate percentages
        for asset_value in &mut asset_values {
            asset_value.percentage_of_portfolio = if total_value_usdc > 0.0 {
                (asset_value.value_usdc / total_value_usdc) * 100.0
            } else {
                0.0
            };
        }

        // Sort by value descending
        asset_values.sort_by(|a, b| b.value_usdc.partial_cmp(&a.value_usdc).unwrap_or(std::cmp::Ordering::Equal));

        let total_value_btc = if btc_price_usdc > 0.0 {
            total_value_usdc / btc_price_usdc
        } else {
            0.0
        };

        let snapshot = PortfolioSnapshot {
            timestamp: Utc::now(),
            run_number,
            btc_price_usdc,
            total_value_usdc,
            total_value_btc,
            assets: asset_values,
            asset_count: user_assets.len(),
        };

        info!("💰 Portfolio snapshot: ${:.2} USDC ({:.6} BTC) across {} assets", 
              snapshot.total_value_usdc, snapshot.total_value_btc, snapshot.asset_count);

        Ok(snapshot)
    }

    async fn calculate_asset_value(&self, asset: &UserAsset, btc_price_usdc: f64) -> Result<AssetValue> {
        let price_usdc = if asset.asset == "USDC" {
            1.0
        } else {
            self.get_asset_price_in_usdc(&asset.asset).await.unwrap_or(0.0)
        };

        let value_usdc = asset.total * price_usdc;
        
        let price_btc = if btc_price_usdc > 0.0 && price_usdc > 0.0 {
            price_usdc / btc_price_usdc
        } else {
            0.0
        };

        let value_btc = if btc_price_usdc > 0.0 {
            value_usdc / btc_price_usdc
        } else {
            0.0
        };

        Ok(AssetValue {
            asset: asset.asset.clone(),
            balance: asset.total,
            price_usdc,
            value_usdc,
            price_btc,
            value_btc,
            percentage_of_portfolio: 0.0, // Will be calculated later
        })
    }

    async fn get_asset_price_in_usdc(&self, asset: &str) -> Result<f64> {
        if asset == "USDC" {
            return Ok(1.0);
        }

        // Try direct USDC pair first
        let symbol = format!("{}USDC", asset);
        if let Ok(klines) = fetch_klines_cached(&symbol, "1m", 1).await {
            if let Some(latest) = klines.last() {
                return Ok(latest.close);
            }
        }

        // Try USDT pair as fallback (assuming USDT ≈ USDC)
        let symbol_usdt = format!("{}USDT", asset);
        if let Ok(klines) = fetch_klines_cached(&symbol_usdt, "1m", 1).await {
            if let Some(latest) = klines.last() {
                return Ok(latest.close);
            }
        }

        // For BTC, try BTCUSDC directly
        if asset == "BTC" {
            if let Ok(klines) = fetch_klines_cached("BTCUSDC", "1m", 1).await {
                if let Some(latest) = klines.last() {
                    return Ok(latest.close);
                }
            }
        }

        error!("Could not find price for asset: {}", asset);
        Ok(0.0)
    }

    pub fn save_portfolio_snapshot(&self, snapshot: &PortfolioSnapshot) -> Result<()> {
        let json_line = serde_json::to_string(snapshot)
            .wrap_err("Failed to serialize portfolio snapshot")?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("data/portfolio.jsonl")
            .wrap_err("Failed to open portfolio.jsonl file")?;

        writeln!(file, "{}", json_line)
            .wrap_err("Failed to write portfolio snapshot to file")?;

        info!("💾 Portfolio snapshot saved to data/portfolio.jsonl");
        Ok(())
    }

    pub async fn track_and_save_portfolio(&self, run_number: u32) -> Result<PortfolioSnapshot> {
        let snapshot = self.capture_portfolio_snapshot(run_number).await?;
        self.save_portfolio_snapshot(&snapshot)?;
        Ok(snapshot)
    }
}

// Utility functions for reading portfolio data
pub fn load_portfolio_history() -> Result<Vec<PortfolioSnapshot>> {
    use std::fs;
    use std::io::{BufRead, BufReader};

    let file_path = "data/portfolio.jsonl";
    
    if !std::path::Path::new(file_path).exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(file_path)
        .wrap_err("Failed to open portfolio.jsonl")?;
    
    let reader = BufReader::new(file);
    let mut snapshots = Vec::new();

    for line in reader.lines() {
        let line = line.wrap_err("Failed to read line from portfolio.jsonl")?;
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<PortfolioSnapshot>(&line) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(e) => {
                error!("Failed to parse portfolio snapshot: {} - Line: {}", e, line);
            }
        }
    }

    // Sort by timestamp
    snapshots.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    Ok(snapshots)
}

pub fn get_portfolio_summary(snapshots: &[PortfolioSnapshot]) -> Option<PortfolioSummary> {
    if snapshots.is_empty() {
        return None;
    }

    let latest = snapshots.last()?;
    let first = snapshots.first()?;

    let value_change_usdc = latest.total_value_usdc - first.total_value_usdc;
    let value_change_percent = if first.total_value_usdc > 0.0 {
        (value_change_usdc / first.total_value_usdc) * 100.0
    } else {
        0.0
    };

    let btc_change_percent = if first.btc_price_usdc > 0.0 {
        ((latest.btc_price_usdc - first.btc_price_usdc) / first.btc_price_usdc) * 100.0
    } else {
        0.0
    };

    Some(PortfolioSummary {
        total_snapshots: snapshots.len(),
        first_snapshot: first.timestamp,
        latest_snapshot: latest.timestamp,
        current_value_usdc: latest.total_value_usdc,
        current_value_btc: latest.total_value_btc,
        value_change_usdc,
        value_change_percent,
        current_btc_price: latest.btc_price_usdc,
        btc_change_percent,
        current_asset_count: latest.asset_count,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_snapshots: usize,
    pub first_snapshot: DateTime<Utc>,
    pub latest_snapshot: DateTime<Utc>,
    pub current_value_usdc: f64,
    pub current_value_btc: f64,
    pub value_change_usdc: f64,
    pub value_change_percent: f64,
    pub current_btc_price: f64,
    pub btc_change_percent: f64,
    pub current_asset_count: usize,
}