use mono_ai_macros::tool;
use crate::tools::get_prices::{get_klines_for_indicators, Kline};

/// Calculate On-Balance Volume (OBV)
pub fn calculate_obv(klines: &[Kline]) -> Vec<f64> {
    if klines.is_empty() {
        return Vec::new();
    }

    let mut obv_values = Vec::new();
    let mut obv = 0.0;
    
    // First OBV value is just the first volume
    obv = klines[0].volume;
    obv_values.push(obv);
    
    // Calculate OBV for subsequent periods
    for i in 1..klines.len() {
        if klines[i].close > klines[i - 1].close {
            // Price up: add volume
            obv += klines[i].volume;
        } else if klines[i].close < klines[i - 1].close {
            // Price down: subtract volume
            obv -= klines[i].volume;
        }
        // Price unchanged: OBV unchanged
        obv_values.push(obv);
    }
    
    obv_values
}

/// Calculate Money Flow Index (MFI)
pub fn calculate_mfi(klines: &[Kline], period: usize) -> Vec<f64> {
    if klines.len() < period + 1 {
        return Vec::new();
    }

    let mut mfi_values = Vec::new();
    
    for i in period..klines.len() {
        let mut positive_flow = 0.0;
        let mut negative_flow = 0.0;
        
        for j in i - period + 1..=i {
            // Calculate typical price
            let typical_price = (klines[j].high + klines[j].low + klines[j].close) / 3.0;
            let prev_typical_price = (klines[j - 1].high + klines[j - 1].low + klines[j - 1].close) / 3.0;
            
            // Calculate money flow
            let money_flow = typical_price * klines[j].volume;
            
            if typical_price > prev_typical_price {
                positive_flow += money_flow;
            } else if typical_price < prev_typical_price {
                negative_flow += money_flow;
            }
        }
        
        // Calculate MFI
        let mfi = if negative_flow == 0.0 {
            100.0
        } else {
            let money_flow_ratio = positive_flow / negative_flow;
            100.0 - (100.0 / (1.0 + money_flow_ratio))
        };
        
        mfi_values.push(mfi);
    }
    
    mfi_values
}

/// Calculate Volume Weighted Average Price (VWAP)
pub fn calculate_vwap(klines: &[Kline]) -> Vec<f64> {
    let mut vwap_values = Vec::new();
    let mut cumulative_volume = 0.0;
    let mut cumulative_pv = 0.0; // Price * Volume
    
    for kline in klines {
        let typical_price = (kline.high + kline.low + kline.close) / 3.0;
        cumulative_pv += typical_price * kline.volume;
        cumulative_volume += kline.volume;
        
        let vwap = if cumulative_volume > 0.0 {
            cumulative_pv / cumulative_volume
        } else {
            typical_price
        };
        
        vwap_values.push(vwap);
    }
    
    vwap_values
}

pub fn format_volume_analysis(
    obv_values: &[f64],
    mfi_values: &[f64],
    vwap_values: &[f64],
    klines: &[Kline],
    symbol: &str,
    mfi_period: usize,
) -> String {
    let mut result = format!("📊 Volume Indicators Analysis for {}:\n\n", symbol);
    
    if let Some(current_price) = klines.last().map(|k| k.close) {
        result.push_str(&format!("💰 Current Price: ${:.4}\n", current_price));
        
        if let Some(current_volume) = klines.last().map(|k| k.volume) {
            result.push_str(&format!("📊 Current Volume: {:.2}\n\n", current_volume));
        }
        
        // OBV Analysis
        if let Some(current_obv) = obv_values.last() {
            result.push_str(&format!("📈 On-Balance Volume (OBV): {:.2}\n", current_obv));
            
            // OBV trend analysis
            if obv_values.len() >= 10 {
                let recent_obv: f64 = obv_values.iter().rev().take(5).sum::<f64>() / 5.0;
                let older_obv: f64 = obv_values.iter().rev().skip(5).take(5).sum::<f64>() / 5.0;
                
                if recent_obv > older_obv * 1.05 {
                    result.push_str("  • 📈 OBV RISING - Volume supporting price trend\n");
                } else if recent_obv < older_obv * 0.95 {
                    result.push_str("  • 📉 OBV FALLING - Volume diverging from price\n");
                } else {
                    result.push_str("  • ➡️ OBV STABLE - Neutral volume trend\n");
                }
            }
        }
        
        // MFI Analysis
        if let Some(current_mfi) = mfi_values.last() {
            result.push_str(&format!("\n💰 Money Flow Index ({}): {:.2}\n", mfi_period, current_mfi));
            
            if *current_mfi > 80.0 {
                result.push_str("  • 🔴 OVERBOUGHT - High buying pressure, consider selling\n");
            } else if *current_mfi < 20.0 {
                result.push_str("  • 🟢 OVERSOLD - Low buying pressure, consider buying\n");
            } else if *current_mfi > 50.0 {
                result.push_str("  • 📈 BULLISH - Money flowing into the asset\n");
            } else {
                result.push_str("  • 📉 BEARISH - Money flowing out of the asset\n");
            }
        }
        
        // VWAP Analysis
        if let Some(current_vwap) = vwap_values.last() {
            result.push_str(&format!("\n📊 VWAP: ${:.4}\n", current_vwap));
            
            if current_price > *current_vwap {
                result.push_str("  • 📈 ABOVE VWAP - Price trading above average, bullish\n");
            } else {
                result.push_str("  • 📉 BELOW VWAP - Price trading below average, bearish\n");
            }
            
            let vwap_distance = ((current_price - current_vwap) / current_vwap) * 100.0;
            result.push_str(&format!("  • 📏 Distance from VWAP: {:.2}%\n", vwap_distance));
        }
        
        // Combined volume signals
        result.push_str("\n🎯 Volume-Based Trading Signals:\n");
        
        if let (Some(obv), Some(mfi), Some(vwap)) = (obv_values.last(), mfi_values.last(), vwap_values.last()) {
            let mut bullish_signals = 0;
            let mut bearish_signals = 0;
            
            // OBV trend
            if obv_values.len() >= 2 {
                let prev_obv = obv_values[obv_values.len() - 2];
                if *obv > prev_obv {
                    bullish_signals += 1;
                } else if *obv < prev_obv {
                    bearish_signals += 1;
                }
            }
            
            // MFI signals
            if *mfi > 50.0 {
                bullish_signals += 1;
            } else {
                bearish_signals += 1;
            }
            
            // VWAP signals
            if current_price > *vwap {
                bullish_signals += 1;
            } else {
                bearish_signals += 1;
            }
            
            if bullish_signals >= 2 {
                result.push_str("  • 🟢 BULLISH VOLUME CONFLUENCE - Multiple volume indicators positive\n");
            } else if bearish_signals >= 2 {
                result.push_str("  • 🔴 BEARISH VOLUME CONFLUENCE - Multiple volume indicators negative\n");
            } else {
                result.push_str("  • 🟡 MIXED VOLUME SIGNALS - No clear volume direction\n");
            }
        }
        
        // Volume spike detection
        if klines.len() >= 20 {
            let recent_volumes: Vec<f64> = klines.iter().rev().take(20).map(|k| k.volume).collect();
            let avg_volume: f64 = recent_volumes.iter().sum::<f64>() / recent_volumes.len() as f64;
            let current_volume = match klines.last() {
                Some(kline) => kline.volume,
                None => return "❌ Error: No kline data available for volume analysis".to_string(),
            };
            
            if current_volume > avg_volume * 2.0 {
                result.push_str("  • 🔥 VOLUME SPIKE - Unusually high volume detected\n");
            } else if current_volume < avg_volume * 0.5 {
                result.push_str("  • 😴 LOW VOLUME - Below average trading activity\n");
            }
        }
    }
    
    // Show recent volume indicator values
    result.push_str("\n🕐 Recent Volume Indicator Values:\n");
    let start_idx = if mfi_values.len() > 5 { mfi_values.len() - 5 } else { 0 };
    
    for i in start_idx..mfi_values.len() {
        let idx = i - start_idx + 1;
        let obv_idx = if obv_values.len() > mfi_values.len() {
            i + (obv_values.len() - mfi_values.len())
        } else {
            i
        };
        
        if let (Some(obv), Some(mfi), Some(vwap)) = (
            obv_values.get(obv_idx),
            mfi_values.get(i),
            vwap_values.get(obv_idx),
        ) {
            let mfi_signal = if *mfi > 80.0 { "🔴" } else if *mfi < 20.0 { "🟢" } else { "⚪" };
            result.push_str(&format!(
                "  {}. OBV: {:.0} | MFI: {:.1} | VWAP: ${:.4} {}\n",
                idx, obv, mfi, vwap, mfi_signal
            ));
        }
    }
    
    result
}

#[tracing::instrument]
#[tool]
/// Calculate volume indicators (OBV, MFI, VWAP) for volume analysis
pub fn calculate_volume_indicators(symbol: String, mfi_period: Option<String>) -> String {
    let mfi_period = mfi_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(14) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let obv_values = calculate_obv(&klines);
                let mfi_values = calculate_mfi(&klines, mfi_period);
                let vwap_values = calculate_vwap(&klines);
                
                format_volume_analysis(&obv_values, &mfi_values, &vwap_values, &klines, &symbol, mfi_period)
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tracing::instrument]
#[tool]
/// Calculate volume indicators with 24-hour data for comprehensive analysis
pub fn calculate_volume_indicators_24h(symbol: String, mfi_period: Option<u32>) -> String {
    let mfi_period = mfi_period.unwrap_or(14) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 288).await { // 24h data
            Ok(klines) => {
                let obv_values = calculate_obv(&klines);
                let mfi_values = calculate_mfi(&klines, mfi_period);
                let vwap_values = calculate_vwap(&klines);
                
                let mut result = format_volume_analysis(&obv_values, &mfi_values, &vwap_values, &klines, &symbol, mfi_period);
                result.push_str("\n📊 Based on 24-hour data (288 candles)\n");
                result
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
    fn test_obv_calculation() {
        let klines = vec![
            Kline { close: 100.0, volume: 1000.0, ..Default::default() },
            Kline { close: 105.0, volume: 1500.0, ..Default::default() }, // Price up
            Kline { close: 102.0, volume: 800.0, ..Default::default() },  // Price down
            Kline { close: 102.0, volume: 1200.0, ..Default::default() }, // Price same
        ];

        let obv_values = calculate_obv(&klines);
        assert_eq!(obv_values.len(), 4);
        assert_eq!(obv_values[0], 1000.0);
        assert_eq!(obv_values[1], 2500.0); // 1000 + 1500
        assert_eq!(obv_values[2], 1700.0); // 2500 - 800
        assert_eq!(obv_values[3], 1700.0); // unchanged
    }

    #[test]
    fn test_vwap_calculation() {
        let klines = vec![
            Kline { high: 110.0, low: 100.0, close: 105.0, volume: 1000.0, ..Default::default() },
            Kline { high: 115.0, low: 105.0, close: 110.0, volume: 1500.0, ..Default::default() },
        ];

        let vwap_values = calculate_vwap(&klines);
        assert_eq!(vwap_values.len(), 2);
        
        // First VWAP should be the typical price of first candle
        let first_typical = (110.0 + 100.0 + 105.0) / 3.0;
        assert!((vwap_values[0] - first_typical).abs() < 0.001);
    }

    #[test]
    fn test_mfi_calculation() {
        let klines = vec![
            Kline { high: 110.0, low: 100.0, close: 105.0, volume: 1000.0, ..Default::default() },
            Kline { high: 115.0, low: 105.0, close: 110.0, volume: 1500.0, ..Default::default() },
            Kline { high: 120.0, low: 110.0, close: 115.0, volume: 1200.0, ..Default::default() },
            Kline { high: 118.0, low: 112.0, close: 114.0, volume: 800.0, ..Default::default() },
        ];

        let mfi_values = calculate_mfi(&klines, 3);
        assert!(!mfi_values.is_empty());
        
        // MFI should be between 0 and 100
        for mfi in &mfi_values {
            assert!(*mfi >= 0.0 && *mfi <= 100.0);
        }
    }
}