use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, eyre, WrapErr};
use polars::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{Level, debug, error, info, instrument, trace, warn};

use crate::config::Config;
use crate::symbol::Symbol;
use crate::utils::date_to_timestamp::date_string_to_timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineData {
    pub open_time: i64,
    pub close_time: i64,
    pub symbol: Symbol,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub quote_asset_volume: f64,
    pub number_of_trades: i64,
    pub taker_buy_base_asset_volume: f64,
    pub taker_buy_quote_asset_volume: f64,
}

#[derive(Debug, Deserialize)]
struct BinanceKline {
    #[serde(rename = "0")]
    open_time: i64,
    #[serde(rename = "1")]
    open: String,
    #[serde(rename = "2")]
    high: String,
    #[serde(rename = "3")]
    low: String,
    #[serde(rename = "4")]
    close: String,
    #[serde(rename = "5")]
    volume: String,
    #[serde(rename = "6")]
    close_time: i64,
    #[serde(rename = "7")]
    quote_asset_volume: String,
    #[serde(rename = "8")]
    number_of_trades: i64,
    #[serde(rename = "9")]
    taker_buy_base_asset_volume: String,
    #[serde(rename = "10")]
    taker_buy_quote_asset_volume: String,
    #[serde(rename = "11")]
    _ignore: String,
}

#[derive(Debug,Clone)]

pub struct DataCollector {
    client: Client,
    data_dir: PathBuf,
    config: Arc<Config>,
}

impl DataCollector {
    pub fn new(config: Arc<Config>) -> color_eyre::Result<Self> {
        let client = Client::new();
        
        let data_dir = PathBuf::from("data");
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)
                .wrap_err("Failed to create data directory")?;
        }

        Ok(DataCollector {
            client,
            data_dir,
            config,
        })
    }

    /// Get the last timestamp from an existing Arrow file
    pub async fn get_last_timestamp(&self, symbol: &str) -> color_eyre::Result<Option<i64>> {
        let file_path = self.get_arrow_file_path(symbol);
        
        if !file_path.exists() {
            return Ok(None);
        }

        let file_path_clone = file_path.clone();
        let symbol_clone = symbol.to_string();
        
        let df = tokio::task::spawn_blocking(move || {
            LazyFrame::scan_ipc(PlPath::Local(Arc::from(file_path_clone.as_path())), Default::default())
                .wrap_err("Failed to scan IPC file")?
                .select([col("close_time")])
                .sort(["close_time"], SortMultipleOptions::default())
                .tail(1)
                .collect()
                .wrap_err("Failed to collect DataFrame")
        })
        .await
        .wrap_err("Failed to execute blocking task")?
        .wrap_err_with(|| format!("Failed to process DataFrame for {}", symbol_clone))?;

        if df.height() == 0 {
            return Ok(None);
        }

        let last_timestamp = df
            .column("close_time")
            .wrap_err("Failed to get close_time column")?
            .i64()
            .wrap_err("Failed to cast close_time to i64")?
            .get(0)
            .ok_or_else(|| eyre!("Failed to get last timestamp"))?;

        Ok(Some(last_timestamp))
    }

    /// Get the Arrow file path for a symbol
    fn get_arrow_file_path(&self, symbol: &str) -> PathBuf {
        self.data_dir.join(format!("{}_5m.arrow", symbol.to_lowercase()))
    }

    /// Fetch klines data from Binance API using reqwest
    pub async fn fetch_klines(
        &self,
        symbol: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> color_eyre::Result<Vec<KlineData>> {
        let start_str = start_time
            .map(|t| DateTime::<Utc>::from_timestamp_millis(t).unwrap().to_rfc3339())
            .unwrap_or_else(|| "recent data".to_string());
        let end_str = end_time
            .map(|t| DateTime::<Utc>::from_timestamp_millis(t).unwrap().to_rfc3339())
            .unwrap_or_else(|| "current time".to_string());
        
        debug!("Fetching klines for {} from {} to {}", symbol, start_str, end_str);

        let mut all_klines = Vec::new();
        let mut current_start = start_time;
        let limit = 1000; // Maximum allowed by Binance

        loop {
            // Add rate limiting to avoid hitting API limits
            sleep(Duration::from_millis(100)).await;
            
            // Build the URL for Binance API
            let mut url = format!(
                "https://api.binance.com/api/v3/klines?symbol={}&interval=5m&limit={}",
                symbol, limit
            );
            
            if let Some(start) = current_start {
                url.push_str(&format!("&startTime={}", start));
            }
            
            if let Some(end) = end_time {
                url.push_str(&format!("&endTime={}", end));
            }
            
            trace!("Requesting: {}", url);
            
            let response = self.client
                .get(&url)
                .send()
                .await
                .wrap_err_with(|| format!("Failed to send request to Binance API for {}", symbol))?;
            
            if !response.status().is_success() {
                return Err(eyre!(
                    "Binance API returned error status {} for symbol {}",
                    response.status(),
                    symbol
                ));
            }
            
            let klines: Vec<BinanceKline> = response
                .json()
                .await
                .wrap_err_with(|| format!("Failed to parse JSON response for {}", symbol))?;

            if klines.is_empty() {
                break;
            }

            let mut batch_klines = Vec::new();
            let klines_len = klines.len();
            for kline in klines {
                let kline_data = KlineData {
                    open_time: kline.open_time,
                    close_time: kline.close_time,
                    symbol: symbol.into(),
                    open: kline.open.parse()?,
                    high: kline.high.parse()?,
                    low: kline.low.parse()?,
                    close: kline.close.parse()?,
                    volume: kline.volume.parse()?,
                    quote_asset_volume: kline.quote_asset_volume.parse()?,
                    number_of_trades: kline.number_of_trades,
                    taker_buy_base_asset_volume: kline.taker_buy_base_asset_volume.parse()?,
                    taker_buy_quote_asset_volume: kline.taker_buy_quote_asset_volume.parse()?,
                };
                batch_klines.push(kline_data);
            }

            // Update current_start for next iteration
            if let Some(last_kline) = batch_klines.last() {
                current_start = Some(last_kline.close_time + 1);
            }

            all_klines.extend(batch_klines);

            // If we got less than the limit, we've reached the end
            if klines_len < limit as usize {
                break;
            }

            // If we have an end_time and we've passed it, break
            if let Some(end_time) = end_time {
                if current_start.unwrap_or(0) >= end_time {
                    break;
                }
            }
        }

        debug!("Fetched {} klines for {}", all_klines.len(), symbol);
        Ok(all_klines)
    }

    /// Convert klines data to Polars DataFrame
    fn klines_to_dataframe(&self, klines: Vec<KlineData>) -> Result<DataFrame> {
        if klines.is_empty() {
            return Ok(DataFrame::empty());
        }

        let open_times: Vec<i64> = klines.iter().map(|k| k.open_time).collect();
        let close_times: Vec<i64> = klines.iter().map(|k| k.close_time).collect();
        let symbols: Vec<String> = klines.iter().map(|k| k.symbol.to_string().clone()).collect();
        let opens: Vec<f64> = klines.iter().map(|k| k.open).collect();
        let highs: Vec<f64> = klines.iter().map(|k| k.high).collect();
        let lows: Vec<f64> = klines.iter().map(|k| k.low).collect();
        let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
        let volumes: Vec<f64> = klines.iter().map(|k| k.volume).collect();
        let quote_volumes: Vec<f64> = klines.iter().map(|k| k.quote_asset_volume).collect();
        let trade_counts: Vec<i64> = klines.iter().map(|k| k.number_of_trades).collect();
        let taker_buy_base_volumes: Vec<f64> = klines.iter().map(|k| k.taker_buy_base_asset_volume).collect();
        let taker_buy_quote_volumes: Vec<f64> = klines.iter().map(|k| k.taker_buy_quote_asset_volume).collect();

        let df = df! [
            "open_time" => open_times,
            "close_time" => close_times,
            "symbol" => symbols,
            "open" => opens,
            "high" => highs,
            "low" => lows,
            "close" => closes,
            "volume" => volumes,
            "quote_asset_volume" => quote_volumes,
            "number_of_trades" => trade_counts,
            "taker_buy_base_asset_volume" => taker_buy_base_volumes,
            "taker_buy_quote_asset_volume" => taker_buy_quote_volumes,
        ]?;

        Ok(df)
    }

    /// Save DataFrame to Arrow file
    pub fn save_to_arrow(&self, df: DataFrame, symbol: &str) -> Result<()> {
        let file_path = self.get_arrow_file_path(symbol);
        
        let mut file = std::fs::File::create(&file_path)
            .wrap_err_with(|| format!("Failed to create Arrow file: {}", file_path.display()))?;
        IpcWriter::new(&mut file).finish(&mut df.clone())
            .wrap_err("Failed to write DataFrame to Arrow file")?;
        
        debug!("Saved {} rows to {}", df.height(), file_path.display());
        Ok(())
    }

    /// Append new data to existing Arrow file
    pub async fn append_to_arrow(&self, new_df: DataFrame, symbol: &str) -> color_eyre::Result<()> {
        let file_path = self.get_arrow_file_path(symbol);
        
        if !file_path.exists() {
            return self.save_to_arrow(new_df, symbol);
        }

        let file_path_clone = file_path.clone();
        let symbol_clone = symbol.to_string();
        
        let combined_df = tokio::task::spawn_blocking(move || {
            // Read existing data
            let existing_df = LazyFrame::scan_ipc(PlPath::Local(Arc::from(file_path_clone.as_path())), Default::default())
                .wrap_err("Failed to scan existing Arrow file")?
                .collect()
                .wrap_err("Failed to collect existing DataFrame")?;

            // Combine with new data and remove duplicates
            concat([existing_df.lazy(), new_df.lazy()], Default::default())
                .wrap_err("Failed to concatenate DataFrames")?
                .unique(None, UniqueKeepStrategy::First)
                .sort(["open_time"], SortMultipleOptions::default())
                .collect()
                .wrap_err("Failed to collect combined DataFrame")
        })
        .await
        .wrap_err("Failed to execute blocking task")?
        .wrap_err_with(|| format!("Failed to process DataFrames for {}", symbol_clone))?;

        // Save the combined data
        self.save_to_arrow(combined_df, symbol)
    }

    #[instrument]
    /// Collect data for a single symbol
    pub async fn collect_symbol_data(&self, symbol: &str) -> Result<()> {
        let span = tracing::span!(Level::DEBUG, "collecting data", symbol=symbol);
        let _enter = span.enter();
        // info!("Starting data collection for {}", symbol);

        // Get the last timestamp from existing data
        let last_timestamp = self.get_last_timestamp(symbol).await?;
        
        let start_time = match last_timestamp {
            Some(ts) => {
                debug!("Found existing data .last timestamp: {}", 
                      DateTime::<Utc>::from_timestamp_millis(ts).unwrap().to_rfc3339());
                Some(ts + 1) // Start from the next millisecond
            }
            None => {
                // Use backtest_start_date when no existing data
                if !self.config.backtest_start_date.is_empty() {
                    match date_string_to_timestamp(&self.config.backtest_start_date) {
                        Ok(start_timestamp) => {
                            let start_ms = start_timestamp * 1000; // Convert to milliseconds
                            debug!("No existing data found, starting from configured backtest_start_date: {}",
                                  
                                  DateTime::<Utc>::from_timestamp_millis(start_ms).unwrap().to_rfc3339());
                            Some(start_ms)
                        }
                        Err(e) => {
                            warn!("Failed to parse backtest_start_date '{}': {}. Fetching recent data instead.",
                                  self.config.backtest_start_date, e);
                            None
                        }
                    }
                } else {
                    info!("No existing data found for {} and no backtest_start_date configured, fetching recent data", symbol);
                    None
                }
            }
        };

        // Fetch new klines data
        let klines = self.fetch_klines(symbol, start_time, None).await?;
        
        if klines.is_empty() {
            info!("No new data available for {}", symbol);
            return Ok(());
        }

        // Convert to DataFrame
        let df = self.klines_to_dataframe(klines)?;
        
        // Save or append to Arrow file
        if last_timestamp.is_some() {
            self.append_to_arrow(df, symbol).await?;
        } else {
            self.save_to_arrow(df, symbol)?;
        }

        debug!("Successfully updated data for {}", symbol);
        Ok(())
    }
#[instrument]
    /// Collect data for all configured pairs
    pub async fn collect_all_data(&self) -> Result<()> {
        debug!("Starting data collection for all pairs");
        
        let pairs = self.config.pairs();
        let mut errors = Vec::new();

        for symbol in pairs {
            match self.collect_symbol_data(&symbol).await {
                Ok(()) => {
                    debug!("Successfully collected data for {}", symbol);
                }
                Err(e) => {
                    error!("Failed to collect data for {}: {}", symbol, e);
                    errors.push((symbol, e));
                }
            }
            
            // Add a small delay between symbols to be respectful to the API
            sleep(Duration::from_millis(200)).await;
        }

        if !errors.is_empty() {
            warn!("Encountered {} errors during data collection", errors.len());
            for (symbol, error) in errors {
                warn!("Error for {}: {}", symbol, error);
            }
        }

        debug!("Data collection completed");
        Ok(())
    }
    #[instrument(level="debug")]
    /// Get klines data for a specific symbol from stored Arrow files
    pub async fn get_klines_for_symbol(&self, symbol: String) -> color_eyre::Result<Vec<KlineData>> {
        let file_path = self.get_arrow_file_path(&symbol);
        
        if !file_path.exists() {
            return Err(eyre!("No data file found for symbol: {}", symbol));
        }

        debug!("Loading klines data for {} from {}", symbol, file_path.display());
        
        let file_path_clone = file_path.clone();
        let symbol_clone = symbol.clone();
        
        let df = tokio::task::spawn_blocking(move || {
            LazyFrame::scan_ipc(PlPath::Local(Arc::from(file_path_clone.as_path())), Default::default())
                .wrap_err("Failed to scan Arrow file")?
                .sort(["open_time"], SortMultipleOptions::default())
                .collect()
                .wrap_err("Failed to collect DataFrame")
        })
        .await
        .wrap_err("Failed to execute blocking task")?
        .wrap_err_with(|| format!("Failed to load DataFrame for {}", symbol_clone))?;

        debug!("Loaded {} klines for {}", df.height(), symbol);
        
        // Convert DataFrame back to Vec<KlineData>
        self.dataframe_to_klines(df)
    }
#[instrument(level="debug")]
    /// Convert DataFrame back to Vec<KlineData>
    fn dataframe_to_klines(&self, df: DataFrame) -> Result<Vec<KlineData>> {
        let mut klines = Vec::new();
        
        let open_times = df.column("open_time")
            .wrap_err("Failed to get open_time column")?
            .i64()
            .wrap_err("Failed to convert open_time to i64")?;
        let close_times = df.column("close_time")
            .wrap_err("Failed to get close_time column")?
            .i64()
            .wrap_err("Failed to convert close_time to i64")?;
        let symbols = df.column("symbol")
            .wrap_err("Failed to get symbol column")?
            .str()
            .wrap_err("Failed to convert symbol to string")?;
        let opens = df.column("open")
            .wrap_err("Failed to get open column")?
            .f64()
            .wrap_err("Failed to convert open to f64")?;
        let highs = df.column("high")
            .wrap_err("Failed to get high column")?
            .f64()
            .wrap_err("Failed to convert high to f64")?;
        let lows = df.column("low")
            .wrap_err("Failed to get low column")?
            .f64()
            .wrap_err("Failed to convert low to f64")?;
        let closes = df.column("close")
            .wrap_err("Failed to get close column")?
            .f64()
            .wrap_err("Failed to convert close to f64")?;
        let volumes = df.column("volume")
            .wrap_err("Failed to get volume column")?
            .f64()
            .wrap_err("Failed to convert volume to f64")?;
        let quote_volumes = df.column("quote_asset_volume")
            .wrap_err("Failed to get quote_asset_volume column")?
            .f64()
            .wrap_err("Failed to convert quote_asset_volume to f64")?;
        let trade_counts = df.column("number_of_trades")
            .wrap_err("Failed to get number_of_trades column")?
            .i64()
            .wrap_err("Failed to convert number_of_trades to i64")?;
        let taker_buy_base_volumes = df.column("taker_buy_base_asset_volume")
            .wrap_err("Failed to get taker_buy_base_asset_volume column")?
            .f64()
            .wrap_err("Failed to convert taker_buy_base_asset_volume to f64")?;
        let taker_buy_quote_volumes = df.column("taker_buy_quote_asset_volume")
            .wrap_err("Failed to get taker_buy_quote_asset_volume column")?
            .f64()
            .wrap_err("Failed to convert taker_buy_quote_asset_volume to f64")?;

        for i in 0..df.height() {
            let kline = KlineData {
                open_time: open_times.get(i).unwrap_or(0),
                close_time: close_times.get(i).unwrap_or(0),
                symbol: symbols.get(i).unwrap_or("").to_string().into(),
                open: opens.get(i).unwrap_or(0.0),
                high: highs.get(i).unwrap_or(0.0),
                low: lows.get(i).unwrap_or(0.0),
                close: closes.get(i).unwrap_or(0.0),
                volume: volumes.get(i).unwrap_or(0.0),
                quote_asset_volume: quote_volumes.get(i).unwrap_or(0.0),
                number_of_trades: trade_counts.get(i).unwrap_or(0),
                taker_buy_base_asset_volume: taker_buy_base_volumes.get(i).unwrap_or(0.0),
                taker_buy_quote_asset_volume: taker_buy_quote_volumes.get(i).unwrap_or(0.0),
            };
            klines.push(kline);
        }

        Ok(klines)
    }
#[instrument]
    /// Get statistics about collected data
    pub async fn get_data_stats(&self) -> color_eyre::Result<HashMap<String, (usize, Option<DateTime<Utc>>, Option<DateTime<Utc>>)>> {
        let mut stats = HashMap::new();
        let pairs = self.config.pairs();

        for symbol in pairs {
            let file_path = self.get_arrow_file_path(&symbol);
            
            if !file_path.exists() {
                stats.insert(symbol, (0, None, None));
                continue;
            }

            let file_path_clone = file_path.clone();
            let symbol_clone = symbol.clone();
            
            let df = tokio::task::spawn_blocking(move || {
                LazyFrame::scan_ipc(PlPath::Local(Arc::from(file_path_clone.as_path())), Default::default())
                    .wrap_err("Failed to scan Arrow file for stats")?
                    .select([
                        col("open_time").min().alias("min_time"),
                        col("open_time").max().alias("max_time"),
                        col("open_time").count().alias("count"),
                    ])
                    .collect()
                    .wrap_err("Failed to collect statistics DataFrame")
            })
            .await
            .wrap_err("Failed to execute blocking task")?
            .wrap_err_with(|| format!("Failed to get stats for {}", symbol_clone))?;

            let count = df.column("count")
                .wrap_err("Failed to get count column")?
                .u32()
                .wrap_err("Failed to convert count to u32")?
                .get(0).unwrap_or(0) as usize;
            let min_time = df.column("min_time")
                .wrap_err("Failed to get min_time column")?
                .i64()
                .wrap_err("Failed to convert min_time to i64")?
                .get(0)
                .and_then(|ts| DateTime::<Utc>::from_timestamp_millis(ts));
            let max_time = df.column("max_time")
                .wrap_err("Failed to get max_time column")?
                .i64()
                .wrap_err("Failed to convert max_time to i64")?
                .get(0)
                .and_then(|ts| DateTime::<Utc>::from_timestamp_millis(ts));

            stats.insert(symbol, (count, min_time, max_time));
        }

        Ok(stats)
    }
}