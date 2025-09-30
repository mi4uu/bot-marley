use mono_ai_macros::tool;
use crate::tools::get_prices::{get_klines_for_indicators, Kline};

/// Calculate Simple Moving Average (SMA)
pub fn calculate_sma(closes: &[f64], period: usize) -> Vec<f64> {
    let mut sma_values = Vec::new();
    
    if closes.len() < period {
        return sma_values;
    }

    for i in period..closes.len() {
        let start_idx = i - period + 1;
        let sum: f64 = closes[start_idx..=i].iter().sum();
        sma_values.push(sum / period as f64);
    }

    sma_values
}

/// Calculate Exponential Moving Average (EMA)
pub fn calculate_ema(closes: &[f64], period: usize) -> Vec<f64> {
    let mut ema_values = Vec::new();
    
    if closes.is_empty() {
        return ema_values;
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    
    // First EMA is just the first close price
    ema_values.push(closes[0]);
    
    for i in 1..closes.len() {
        let ema = (closes[i] * multiplier) + (ema_values[i - 1] * (1.0 - multiplier));
        ema_values.push(ema);
    }

    ema_values
}

/// Calculate Weighted Moving Average (WMA)
pub fn calculate_wma(closes: &[f64], period: usize) -> Vec<f64> {
    let mut wma_values = Vec::new();
    
    if closes.len() < period {
        return wma_values;
    }

    let weight_sum: f64 = (1..=period).map(|i| i as f64).sum();

    for i in period..closes.len() {
        let mut weighted_sum = 0.0;
        let start_idx = i - period + 1;
        for j in 0..period {
            weighted_sum += closes[start_idx + j] * (j + 1) as f64;
        }
        wma_values.push(weighted_sum / weight_sum);
    }

    wma_values
}

pub fn format_ma_analysis(
    closes: &[f64],
    sma_values: &[f64],
    ema_values: &[f64],
    wma_values: &[f64],
    symbol: &str,
    period: usize,
) -> String {
    let mut result = format!("📊 Moving Averages Analysis for {} (Period: {}):\n\n", symbol, period);
    
    if let Some(current_price) = closes.last() {
        result.push_str(&format!("💰 Current Price: ${:.4}\n\n", current_price));
        
        if let Some(sma) = sma_values.last() {
            let sma_signal = if *current_price > *sma { "📈 BULLISH" } else { "📉 BEARISH" };
            result.push_str(&format!("📊 SMA({}): ${:.4} - {}\n", period, sma, sma_signal));
        }
        
        if let Some(ema) = ema_values.last() {
            let ema_signal = if *current_price > *ema { "📈 BULLISH" } else { "📉 BEARISH" };
            result.push_str(&format!("⚡ EMA({}): ${:.4} - {}\n", period, ema, ema_signal));
        }
        
        if let Some(wma) = wma_values.last() {
            let wma_signal = if *current_price > *wma { "📈 BULLISH" } else { "📉 BEARISH" };
            result.push_str(&format!("⚖️ WMA({}): ${:.4} - {}\n", period, wma, wma_signal));
        }
    }
    
    // Trading signals
    result.push_str("\n🎯 Trading Signals:\n");
    
    if let (Some(current_price), Some(sma), Some(ema)) = (closes.last(), sma_values.last(), ema_values.last()) {
        let above_sma = *current_price > *sma;
        let above_ema = *current_price > *ema;
        
        match (above_sma, above_ema) {
            (true, true) => result.push_str("  • 🟢 STRONG BUY - Price above both SMA and EMA\n"),
            (false, false) => result.push_str("  • 🔴 STRONG SELL - Price below both SMA and EMA\n"),
            (true, false) => result.push_str("  • 🟡 MIXED - Price above SMA but below EMA\n"),
            (false, true) => result.push_str("  • 🟡 MIXED - Price above EMA but below SMA\n"),
        }
        
        // EMA vs SMA crossover
        if *ema > *sma {
            result.push_str("  • 📈 EMA above SMA - Short-term bullish momentum\n");
        } else {
            result.push_str("  • 📉 EMA below SMA - Short-term bearish momentum\n");
        }
    }
    
    // Show recent values
    result.push_str("\n🕐 Recent Moving Average Values:\n");
    let start_idx = if sma_values.len() > 5 { sma_values.len() - 5 } else { 0 };
    
    for i in start_idx..sma_values.len() {
        let idx = i - start_idx + 1;
        if let (Some(sma), Some(ema)) = (sma_values.get(i), ema_values.get(i)) {
            result.push_str(&format!("  {}. SMA: ${:.4} | EMA: ${:.4}\n", idx, sma, ema));
        }
    }
    
    result
}

#[tool]
/// Calculate multiple moving averages (SMA, EMA, WMA) for technical analysis
pub fn calculate_moving_averages(symbol: String, period: Option<String>) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20) as usize;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                
                let sma_values = calculate_sma(&closes, period);
                let ema_values = calculate_ema(&closes, period);
                let wma_values = calculate_wma(&closes, period);
                
                format_ma_analysis(&closes, &sma_values, &ema_values, &wma_values, &symbol, period)
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate SMA (Simple Moving Average) only
pub fn calculate_sma_indicator(symbol: String, period: Option<String>) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20) as usize;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let sma_values = calculate_sma(&closes, period);
                
                if let (Some(current_price), Some(sma)) = (closes.last(), sma_values.last()) {
                    let signal = if *current_price > *sma { "📈 BULLISH" } else { "📉 BEARISH" };
                    format!(
                        "📊 SMA({}) for {}:\n💰 Current Price: ${:.4}\n📊 SMA: ${:.4}\n🎯 Signal: {}",
                        period, symbol, current_price, sma, signal
                    )
                } else {
                    format!("❌ Insufficient data for SMA calculation for {}", symbol)
                }
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate EMA (Exponential Moving Average) only
pub fn calculate_ema_indicator(symbol: String, period: Option<String>) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20) as usize;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let ema_values = calculate_ema(&closes, period);
                
                if let (Some(current_price), Some(ema)) = (closes.last(), ema_values.last()) {
                    let signal = if *current_price > *ema { "📈 BULLISH" } else { "📉 BEARISH" };
                    format!(
                        "⚡ EMA({}) for {}:\n💰 Current Price: ${:.4}\n⚡ EMA: ${:.4}\n🎯 Signal: {}",
                        period, symbol, current_price, ema, signal
                    )
                } else {
                    format!("❌ Insufficient data for EMA calculation for {}", symbol)
                }
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_calculation() {
        let closes = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sma = calculate_sma(&closes, 3);
        assert_eq!(sma, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_ema_calculation() {
        let closes = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ema = calculate_ema(&closes, 3);
        assert_eq!(ema.len(), 5);
        assert_eq!(ema[0], 1.0); // First EMA equals first price
    }

    #[test]
    fn test_wma_calculation() {
        let closes = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let wma = calculate_wma(&closes, 3);
        assert_eq!(wma.len(), 3);
        // WMA for [1,2,3] = (1*1 + 2*2 + 3*3) / (1+2+3) = 14/6 ≈ 2.33
        assert!((wma[0] - 2.333333).abs() < 0.001);
    }
}