use mono_ai_macros::tool;
use crate::tools::get_prices::{get_klines_for_indicators, Kline};

/// Calculate True Range for a single period
fn calculate_true_range(current: &Kline, previous: &Kline) -> f64 {
    let tr1 = current.high - current.low;
    let tr2 = (current.high - previous.close).abs();
    let tr3 = (current.low - previous.close).abs();
    
    tr1.max(tr2).max(tr3)
}

/// Calculate ATR (Average True Range)
pub fn calculate_atr(klines: &[Kline], period: usize) -> Vec<f64> {
    if klines.len() < period + 1 {
        return Vec::new();
    }

    // Calculate True Range for each period
    let mut true_ranges = Vec::new();
    for i in 1..klines.len() {
        let tr = calculate_true_range(&klines[i], &klines[i - 1]);
        true_ranges.push(tr);
    }

    // Calculate ATR using Simple Moving Average of True Range
    let mut atr_values = Vec::new();
    for i in period - 1..true_ranges.len() {
        let start_idx = if i + 1 >= period { i + 1 - period } else { 0 };
        let sum: f64 = true_ranges[start_idx..=i].iter().sum();
        let count = i - start_idx + 1;
        atr_values.push(sum / count as f64);
    }

    atr_values
}

/// Calculate ATR percentage (ATR / Current Price * 100)
pub fn calculate_atr_percentage(atr_values: &[f64], prices: &[f64]) -> Vec<f64> {
    atr_values.iter()
        .zip(prices.iter())
        .map(|(atr, price)| (atr / price) * 100.0)
        .collect()
}

pub fn format_atr_analysis(
    atr_values: &[f64],
    atr_percentages: &[f64],
    current_price: f64,
    symbol: &str,
    period: usize,
) -> String {
    let mut result = format!("📊 ATR({}) Volatility Analysis for {}:\n\n", period, symbol);
    
    result.push_str(&format!("💰 Current Price: ${:.4}\n", current_price));
    
    if let (Some(current_atr), Some(atr_pct)) = (atr_values.last(), atr_percentages.last()) {
        result.push_str(&format!("📊 Current ATR: ${:.4}\n", current_atr));
        result.push_str(&format!("📈 ATR Percentage: {:.2}%\n\n", atr_pct));
        
        // Volatility interpretation
        result.push_str("🎯 Volatility Analysis:\n");
        
        if *atr_pct > 5.0 {
            result.push_str("  • 🔥 EXTREMELY HIGH VOLATILITY - Major price swings expected\n");
        } else if *atr_pct > 3.0 {
            result.push_str("  • 📈 HIGH VOLATILITY - Significant price movements likely\n");
        } else if *atr_pct > 1.5 {
            result.push_str("  • 📊 MODERATE VOLATILITY - Normal price fluctuations\n");
        } else if *atr_pct > 0.5 {
            result.push_str("  • 📉 LOW VOLATILITY - Limited price movements\n");
        } else {
            result.push_str("  • 😴 VERY LOW VOLATILITY - Minimal price action\n");
        }
        
        // Trading implications
        result.push_str("\n💡 Trading Implications:\n");
        
        if *atr_pct > 3.0 {
            result.push_str("  • ⚠️ Use wider stop losses due to high volatility\n");
            result.push_str("  • 🎯 Good for breakout strategies\n");
            result.push_str("  • 📊 Consider position sizing adjustments\n");
        } else if *atr_pct < 1.0 {
            result.push_str("  • 🎯 Good for mean reversion strategies\n");
            result.push_str("  • 📊 Tighter stop losses may be appropriate\n");
            result.push_str("  • ⏰ Potential breakout setup forming\n");
        } else {
            result.push_str("  • 📊 Normal trading conditions\n");
            result.push_str("  • 🎯 Standard risk management applies\n");
        }
        
        // Support and resistance levels based on ATR
        let support_level = current_price - current_atr;
        let resistance_level = current_price + current_atr;
        
        result.push_str(&format!("\n📊 ATR-Based Levels:\n"));
        result.push_str(&format!("  • 📉 Support Level: ${:.4}\n", support_level));
        result.push_str(&format!("  • 📈 Resistance Level: ${:.4}\n", resistance_level));
        
        // Volatility trend analysis
        if atr_values.len() >= 10 {
            let recent_avg: f64 = atr_values.iter().rev().take(5).sum::<f64>() / 5.0;
            let older_avg: f64 = atr_values.iter().rev().skip(5).take(5).sum::<f64>() / 5.0;
            
            result.push_str("\n📈 Volatility Trend:\n");
            if recent_avg > older_avg * 1.1 {
                result.push_str("  • 📈 INCREASING - Volatility expanding\n");
            } else if recent_avg < older_avg * 0.9 {
                result.push_str("  • 📉 DECREASING - Volatility contracting\n");
            } else {
                result.push_str("  • ➡️ STABLE - Volatility unchanged\n");
            }
        }
    }
    
    // Show recent ATR values
    result.push_str("\n🕐 Recent ATR Values:\n");
    let start_idx = if atr_values.len() > 5 { atr_values.len() - 5 } else { 0 };
    
    for i in start_idx..atr_values.len() {
        let idx = i - start_idx + 1;
        if let (Some(atr), Some(pct)) = (atr_values.get(i), atr_percentages.get(i)) {
            let volatility_level = if *pct > 3.0 { "🔥" } else if *pct > 1.5 { "📊" } else { "📉" };
            result.push_str(&format!(
                "  {}. ATR: ${:.4} ({:.2}%) {}\n",
                idx, atr, pct, volatility_level
            ));
        }
    }
    
    result
}

#[tool]
/// Calculate ATR (Average True Range) for volatility analysis
pub fn calculate_atr_indicator(symbol: String, period: Option<String>) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(14) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let atr_values = calculate_atr(&klines, period);
                
                if let Some(current_price) = klines.last().map(|k| k.close) {
                    let prices: Vec<f64> = klines.iter().skip(period).map(|k| k.close).collect();
                    let atr_percentages = calculate_atr_percentage(&atr_values, &prices);
                    
                    format_atr_analysis(&atr_values, &atr_percentages, current_price, &symbol, period)
                } else {
                    format!("❌ No price data available for {}", symbol)
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch price data for ATR calculation: {}", e);
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate ATR with 24-hour data for comprehensive volatility analysis
pub fn calculate_atr_24h(symbol: String, period: Option<String>) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(14) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 288).await { // 24h data
            Ok(klines) => {
                let atr_values = calculate_atr(&klines, period);
                
                if let Some(current_price) = klines.last().map(|k| k.close) {
                    let prices: Vec<f64> = klines.iter().skip(period).map(|k| k.close).collect();
                    let atr_percentages = calculate_atr_percentage(&atr_values, &prices);
                    
                    let mut result = format_atr_analysis(&atr_values, &atr_percentages, current_price, &symbol, period);
                    result.push_str("\n📊 Based on 24-hour data (288 candles)\n");
                    result
                } else {
                    format!("❌ No price data available for {}", symbol)
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch 24h price data for ATR calculation: {}", e);
                format!("❌ Error fetching 24h price data for {}: {}", symbol, e)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::get_prices::Kline;

    #[test]
    fn test_true_range_calculation() {
        let current = Kline {
            high: 110.0,
            low: 105.0,
            close: 108.0,
            ..Default::default()
        };
        let previous = Kline {
            close: 107.0,
            ..Default::default()
        };
        
        let tr = calculate_true_range(&current, &previous);
        // TR should be max of (110-105=5, |110-107|=3, |105-107|=2) = 5
        assert_eq!(tr, 5.0);
    }

    #[test]
    fn test_atr_calculation() {
        let klines = vec![
            Kline { high: 100.0, low: 95.0, close: 98.0, ..Default::default() },
            Kline { high: 102.0, low: 97.0, close: 100.0, ..Default::default() },
            Kline { high: 105.0, low: 99.0, close: 103.0, ..Default::default() },
            Kline { high: 107.0, low: 102.0, close: 105.0, ..Default::default() },
            Kline { high: 110.0, low: 104.0, close: 108.0, ..Default::default() },
        ];

        let atr_values = calculate_atr(&klines, 3);
        assert!(!atr_values.is_empty());
        assert!(atr_values[0] > 0.0);
    }

    #[test]
    fn test_atr_percentage_calculation() {
        let atr_values = vec![2.0, 2.5, 3.0];
        let prices = vec![100.0, 125.0, 150.0];
        let atr_percentages = calculate_atr_percentage(&atr_values, &prices);
        
        assert_eq!(atr_percentages.len(), 3);
        assert_eq!(atr_percentages[0], 2.0); // 2.0/100.0 * 100 = 2%
        assert_eq!(atr_percentages[1], 2.0); // 2.5/125.0 * 100 = 2%
        assert_eq!(atr_percentages[2], 2.0); // 3.0/150.0 * 100 = 2%
    }
}