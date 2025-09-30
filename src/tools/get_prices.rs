use mono_ai_macros::tool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;
use tracing::{info, debug};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Kline {
    pub open_time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub close_time: u64,
    pub quote_asset_volume: f64,
    pub number_of_trades: u64,
    pub taker_buy_base_asset_volume: f64,
    pub taker_buy_quote_asset_volume: f64,
    pub ignore: f64,
}

impl Kline {
    pub fn from_binance_array(arr: &[serde_json::Value]) -> Result<Self, Box<dyn std::error::Error>> {
        if arr.len() < 12 {
            return Err("Invalid kline array length".into());
        }

        Ok(Kline {
            open_time: arr[0].as_u64().ok_or("Invalid open_time")?,
            open: arr[1].as_str().ok_or("Invalid open")?.parse()?,
            high: arr[2].as_str().ok_or("Invalid high")?.parse()?,
            low: arr[3].as_str().ok_or("Invalid low")?.parse()?,
            close: arr[4].as_str().ok_or("Invalid close")?.parse()?,
            volume: arr[5].as_str().ok_or("Invalid volume")?.parse()?,
            close_time: arr[6].as_u64().ok_or("Invalid close_time")?,
            quote_asset_volume: arr[7].as_str().ok_or("Invalid quote_asset_volume")?.parse()?,
            number_of_trades: arr[8].as_u64().ok_or("Invalid number_of_trades")?,
            taker_buy_base_asset_volume: arr[9].as_str().ok_or("Invalid taker_buy_base_asset_volume")?.parse()?,
            taker_buy_quote_asset_volume: arr[10].as_str().ok_or("Invalid taker_buy_quote_asset_volume")?.parse()?,
            ignore: arr[11].as_str().ok_or("Invalid ignore")?.parse()?,
        })
    }

    pub fn format_for_display(&self) -> String {
        format!(
            "Time: {} | Open: ${:.4} | High: ${:.4} | Low: ${:.4} | Close: ${:.4} | Volume: {:.2}",
            self.format_timestamp(),
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume
        )
    }

    pub fn format_timestamp(&self) -> String {
        use std::time::SystemTime;
        let datetime = SystemTime::UNIX_EPOCH + Duration::from_millis(self.open_time);
        format!("{:?}", datetime)
    }
}

#[derive(Debug, Clone)]
struct CachedKlineData {
    klines: Vec<Kline>,
    timestamp: Instant,
}

impl CachedKlineData {
    fn new(klines: Vec<Kline>) -> Self {
        Self {
            klines,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, cache_duration: Duration) -> bool {
        self.timestamp.elapsed() > cache_duration
    }
}

type KlineCache = Arc<Mutex<HashMap<String, CachedKlineData>>>;

static KLINE_CACHE: OnceCell<KlineCache> = OnceCell::const_new();

async fn get_cache() -> &'static KlineCache {
    KLINE_CACHE.get_or_init(|| async {
        Arc::new(Mutex::new(HashMap::new()))
    }).await
}

pub async fn fetch_klines_cached(symbol: &str, interval: &str, limit: u32) -> Result<Vec<Kline>, Box<dyn std::error::Error>> {
  dbg!(symbol,&interval,&limit);
    let cache_key = format!("{}:{}:{}", symbol, interval, limit);
    let cache_duration = Duration::from_secs(60); // 1 minute cache
    
    let cache = get_cache().await;
    
    // Check cache first
    {
        let cache_guard = cache.lock().unwrap();
        if let Some(cached_data) = cache_guard.get(&cache_key) {
            if !cached_data.is_expired(cache_duration) {
                info!("📋 Using cached data for {}", symbol);
                return Ok(cached_data.klines.clone());
            }
        }
    }

    info!("🌐 Fetching fresh data for {} from Binance API", symbol);
    
    // Fetch fresh data
    let client = Client::new();
    let url = "https://api.binance.com/api/v3/klines";
    let params = [
        ("symbol", symbol),
        ("interval", interval),
        ("limit", &limit.to_string()),
    ];

    let response = client
        .get(url)
        .query(&params)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let klines_array = response.as_array().ok_or("Invalid response format")?;
    let mut klines = Vec::new();

    for kline_data in klines_array {
        let kline_array = kline_data.as_array().ok_or("Invalid kline format")?;
        let kline = Kline::from_binance_array(kline_array)?;
        klines.push(kline);
    }

    // Update cache
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(cache_key, CachedKlineData::new(klines.clone()));
    }

    Ok(klines)
}

pub fn format_klines_for_ai(klines: &[Kline], symbol: &str) -> String {
    let mut result = String::new();
    
    if klines.is_empty() {
        result.push_str("No data available.\n");
        return result;
    }

    // Calculate actual time period covered
    let time_period_hours = (klines.len() * 5) as f64 / 60.0; // 5-minute intervals
    let time_period_str = if time_period_hours >= 24.0 {
        format!("{:.1} days", time_period_hours / 24.0)
    } else {
        format!("{:.1} hours", time_period_hours)
    };

    result.push_str(&format!("📊 Price Data for {} (Last {} candles, ~{}):\n\n", symbol, klines.len(), time_period_str));

    // Current price info
    if let Some(latest) = klines.last() {
        result.push_str(&format!("🔴 CURRENT PRICE: ${:.4}\n", latest.close));
        result.push_str(&format!("📈 Period High: ${:.4}\n", klines.iter().map(|k| k.high).fold(0.0, f64::max)));
        result.push_str(&format!("📉 Period Low: ${:.4}\n", klines.iter().map(|k| k.low).fold(f64::INFINITY, f64::min)));
        result.push_str(&format!("📊 Period Volume: {:.2}\n", klines.iter().map(|k| k.volume).sum::<f64>()));
        
        // Calculate period change
        if let Some(first) = klines.first() {
            let period_change = ((latest.close - first.open) / first.open) * 100.0;
            let change_emoji = if period_change > 0.0 { "📈" } else { "📉" };
            result.push_str(&format!("{} Period Change: {:.2}%\n\n", change_emoji, period_change));
        } else {
            result.push_str("\n");
        }
    }

    // Recent candles (last 10)
    result.push_str("🕐 Recent Candles (newest first):\n");
    for (i, kline) in klines.iter().rev().take(10).enumerate() {
        let change = if i < klines.len() - 1 {
            let prev_close = klines[klines.len() - 2 - i].close;
            let change_pct = ((kline.close - prev_close) / prev_close) * 100.0;
            if change_pct > 0.0 {
                format!(" (+{:.2}%)", change_pct)
            } else {
                format!(" ({:.2}%)", change_pct)
            }
        } else {
            String::new()
        };
        
        result.push_str(&format!("  {}. {}{}\n", i + 1, kline.format_for_display(), change));
    }

    // Price trend analysis
    if klines.len() >= 5 {
        let recent_closes: Vec<f64> = klines.iter().rev().take(5).map(|k| k.close).collect();
        let trend = if recent_closes[0] > recent_closes[4] {
            "📈 UPTREND"
        } else if recent_closes[0] < recent_closes[4] {
            "📉 DOWNTREND"
        } else {
            "➡️ SIDEWAYS"
        };
        result.push_str(&format!("\n🎯 Short-term Trend (5 candles): {}\n", trend));
    }

    result
}

#[tool]
/// Get comprehensive price data for a trading symbol with caching optimization
pub fn get_price(symbol: String) -> String {
    // Use tokio runtime to handle async call in sync context
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // For 24h data, we need 288 candles (24h * 60min / 5min intervals)
        // For now, using 100 candles (~8.33 hours) for faster response
        match fetch_klines_cached(&symbol, "5m", 100).await {
            Ok(klines) => {
                format_klines_for_ai(&klines, &symbol)
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Get 24-hour price data for a trading symbol (288 x 5min candles)
pub fn get_price_24h(symbol: String) -> String {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 288 candles = 24 hours of 5-minute data
        match fetch_klines_cached(&symbol, "5m", 288).await {
            Ok(klines) => {
                format_klines_for_ai(&klines, &symbol)
            }
            Err(e) => {
                format!("❌ Error fetching 24h price data for {}: {}", symbol, e)
            }
        }
    })
}

// Additional utility functions for future indicator calculations
pub async fn get_klines_for_indicators(symbol: &str, interval: &str, limit: u32) -> Result<Vec<Kline>, Box<dyn std::error::Error>> {
    fetch_klines_cached(symbol, interval, limit).await
}

pub fn calculate_sma(klines: &[Kline], period: usize) -> Vec<f64> {
    let mut sma_values = Vec::new();
    
    if klines.len() < period {
        return sma_values;
    }

    for i in period - 1..klines.len() {
        let sum: f64 = klines[i - period + 1..=i].iter().map(|k| k.close).sum();
        sma_values.push(sum / period as f64);
    }

    sma_values
}

pub fn calculate_rsi(klines: &[Kline], period: usize) -> Vec<f64> {
    let mut rsi_values = Vec::new();
    
    if klines.len() < period + 1 {
        return rsi_values;
    }

    let mut gains = Vec::new();
    let mut losses = Vec::new();

    // Calculate price changes
    for i in 1..klines.len() {
        let change = klines[i].close - klines[i - 1].close;
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }
    }

    // Calculate RSI
    for i in period - 1..gains.len() {
        let avg_gain: f64 = gains[i - period + 1..=i].iter().sum::<f64>() / period as f64;
        let avg_loss: f64 = losses[i - period + 1..=i].iter().sum::<f64>() / period as f64;
        
        if avg_loss == 0.0 {
            rsi_values.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }
    }

    rsi_values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_from_binance_array() {
        let test_data = vec![
            serde_json::json!(1640995200000u64),
            serde_json::json!("47000.00"),
            serde_json::json!("47500.00"),
            serde_json::json!("46800.00"),
            serde_json::json!("47200.00"),
            serde_json::json!("100.50"),
            serde_json::json!(1640995499999u64),
            serde_json::json!("4740000.00"),
            serde_json::json!(1500u64),
            serde_json::json!("50.25"),
            serde_json::json!("2370000.00"),
            serde_json::json!("0"),
        ];

        let kline = Kline::from_binance_array(&test_data).unwrap();
        assert_eq!(kline.open, 47000.00);
        assert_eq!(kline.high, 47500.00);
        assert_eq!(kline.low, 46800.00);
        assert_eq!(kline.close, 47200.00);
    }

    #[test]
    fn test_calculate_sma() {
        let klines = vec![
            Kline { close: 10.0, ..Default::default() },
            Kline { close: 20.0, ..Default::default() },
            Kline { close: 30.0, ..Default::default() },
            Kline { close: 40.0, ..Default::default() },
        ];

        let sma = calculate_sma(&klines, 2);
        assert_eq!(sma, vec![15.0, 25.0, 35.0]);
    }
}
