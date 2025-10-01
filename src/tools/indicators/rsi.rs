use mono_ai_macros::tool;
use crate::tools::get_prices::{get_klines_for_indicators, Kline};
use crate::tools::indicators::{IndicatorResult, generate_signals};

/// Calculate RSI (Relative Strength Index) using the financial_indicators crate
pub fn calculate_rsi_from_klines(klines: &[Kline], period: usize) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    if klines.len() < period + 1 {
        return Err("Not enough data for RSI calculation".into());
    }

    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    
    // Calculate RSI manually since we need to handle the financial_indicators crate properly
    let mut gains = Vec::new();
    let mut losses = Vec::new();

    // Calculate price changes
    for i in 1..closes.len() {
        let change = closes[i] - closes[i - 1];
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }
    }

    let mut rsi_values = Vec::new();

    // Calculate RSI for each period
    for i in period..gains.len() {
        let start_idx = i - period + 1;
        let avg_gain: f64 = gains[start_idx..=i].iter().sum::<f64>() / period as f64;
        let avg_loss: f64 = losses[start_idx..=i].iter().sum::<f64>() / period as f64;
        
        if avg_loss == 0.0 {
            rsi_values.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }
    }

    Ok(rsi_values)
}

pub fn format_rsi_analysis(rsi_values: &[f64], symbol: &str, period: usize) -> String {
    if rsi_values.is_empty() {
        return format!("❌ No RSI data available for {}", symbol);
    }

    let latest_rsi = match rsi_values.last() {
        Some(rsi) => rsi,
        None => return "❌ Error: No RSI values calculated".to_string(),
    };
    let mut result = format!("📊 RSI({}) Analysis for {}:\n\n", period, symbol);
    
    result.push_str(&format!("🔢 Current RSI: {:.2}\n", latest_rsi));
    
    // RSI interpretation
    let interpretation = if *latest_rsi > 70.0 {
        "🔴 OVERBOUGHT - Price may decline soon"
    } else if *latest_rsi < 30.0 {
        "🟢 OVERSOLD - Price may rise soon"
    } else if *latest_rsi > 50.0 {
        "📈 Bullish momentum"
    } else {
        "📉 Bearish momentum"
    };
    
    result.push_str(&format!("📈 Signal: {}\n", interpretation));
    
    // Show recent RSI values
    result.push_str("\n🕐 Recent RSI Values:\n");
    let start_idx = if rsi_values.len() > 5 { rsi_values.len() - 5 } else { 0 };
    for (i, &rsi) in rsi_values[start_idx..].iter().enumerate() {
        let status = if rsi > 70.0 { "🔴" } else if rsi < 30.0 { "🟢" } else { "⚪" };
        result.push_str(&format!("  {}. RSI: {:.2} {}\n", i + 1, rsi, status));
    }
    
    // Trading recommendations
    result.push_str("\n🎯 Trading Recommendations:\n");
    if *latest_rsi > 80.0 {
        result.push_str("  • Strong sell signal - Consider taking profits\n");
    } else if *latest_rsi > 70.0 {
        result.push_str("  • Caution - Overbought conditions\n");
    } else if *latest_rsi < 20.0 {
        result.push_str("  • Strong buy signal - Potential reversal\n");
    } else if *latest_rsi < 30.0 {
        result.push_str("  • Consider buying - Oversold conditions\n");
    } else {
        result.push_str("  • Neutral - Wait for clearer signals\n");
    }
    
    result
}

#[tool]
/// Calculate RSI (Relative Strength Index) for technical analysis
pub fn calculate_rsi(symbol: String, period: Option<String>) -> String {
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
                match calculate_rsi_from_klines(&klines, period) {
                    Ok(rsi_values) => {
                        format_rsi_analysis(&rsi_values, &symbol, period)
                    }
                    Err(e) => {
                        format!("❌ Error calculating RSI for {}: {}", symbol, e)
                    }
                }
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate RSI with 24-hour data for more comprehensive analysis
pub fn calculate_rsi_24h(symbol: String, period: Option<String>) -> String {
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
                match calculate_rsi_from_klines(&klines, period) {
                    Ok(rsi_values) => {
                        let mut result = format_rsi_analysis(&rsi_values, &symbol, period);
                        result.push_str("\n📊 Based on 24-hour data (288 candles)\n");
                        result
                    }
                    Err(e) => {
                        format!("❌ Error calculating 24h RSI for {}: {}", symbol, e)
                    }
                }
            }
            Err(e) => {
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
    fn test_rsi_calculation() {
        let klines = vec![
            Kline { close: 44.0, ..Default::default() },
            Kline { close: 44.25, ..Default::default() },
            Kline { close: 44.5, ..Default::default() },
            Kline { close: 43.75, ..Default::default() },
            Kline { close: 44.5, ..Default::default() },
            Kline { close: 44.0, ..Default::default() },
            Kline { close: 44.25, ..Default::default() },
            Kline { close: 44.75, ..Default::default() },
            Kline { close: 45.0, ..Default::default() },
            Kline { close: 45.25, ..Default::default() },
            Kline { close: 45.5, ..Default::default() },
            Kline { close: 45.25, ..Default::default() },
            Kline { close: 45.75, ..Default::default() },
            Kline { close: 46.0, ..Default::default() },
            Kline { close: 46.25, ..Default::default() },
        ];

        let rsi_values = calculate_rsi_from_klines(&klines, 14).expect("Failed to calculate RSI in test");
        assert!(!rsi_values.is_empty());
        assert!(rsi_values[0] >= 0.0 && rsi_values[0] <= 100.0);
    }
}