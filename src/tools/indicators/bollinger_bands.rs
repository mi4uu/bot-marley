use mono_ai_macros::tool;
use crate::tools::get_prices::get_klines_for_indicators;
use crate::tools::indicators::moving_averages::calculate_sma;

#[derive(Debug, Clone)]
pub struct BollingerBandsResult {
    pub upper_band: Vec<f64>,
    pub middle_band: Vec<f64>, // SMA
    pub lower_band: Vec<f64>,
    pub bandwidth: Vec<f64>,
    pub percent_b: Vec<f64>,
}

/// Calculate standard deviation for a given period
fn calculate_std_dev(values: &[f64], period: usize) -> Vec<f64> {
    let mut std_devs = Vec::new();
    
    if values.len() < period {
        return std_devs;
    }

    for i in period..values.len() {
        let start_idx = i - period + 1;
        let slice = &values[start_idx..=i];
        let mean: f64 = slice.iter().sum::<f64>() / period as f64;
        let variance: f64 = slice.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / period as f64;
        std_devs.push(variance.sqrt());
    }

    std_devs
}

/// Calculate Bollinger Bands
pub fn calculate_bollinger_bands(closes: &[f64], period: usize, std_dev_multiplier: f64) -> BollingerBandsResult {
    let middle_band = calculate_sma(closes, period);
    let std_devs = calculate_std_dev(closes, period);
    
    let mut upper_band = Vec::new();
    let mut lower_band = Vec::new();
    let mut bandwidth = Vec::new();
    let mut percent_b = Vec::new();
    
    for i in 0..middle_band.len().min(std_devs.len()) {
        let upper = middle_band[i] + (std_devs[i] * std_dev_multiplier);
        let lower = middle_band[i] - (std_devs[i] * std_dev_multiplier);
        
        upper_band.push(upper);
        lower_band.push(lower);
        
        // Bandwidth: (Upper Band - Lower Band) / Middle Band
        let bw = (upper - lower) / middle_band[i];
        bandwidth.push(bw);
        
        // %B: (Price - Lower Band) / (Upper Band - Lower Band)
        if let Some(current_price) = closes.get(i + period - 1) {
            let pb = (current_price - lower) / (upper - lower);
            percent_b.push(pb);
        }
    }
    
    BollingerBandsResult {
        upper_band,
        middle_band,
        lower_band,
        bandwidth,
        percent_b,
    }
}

pub fn format_bollinger_bands_analysis(
    bb_result: &BollingerBandsResult,
    current_price: f64,
    symbol: &str,
    period: usize,
    std_dev_multiplier: f64,
) -> String {
    let mut result = format!(
        "📊 Bollinger Bands({}, {:.1}) Analysis for {}:\n\n",
        period, std_dev_multiplier, symbol
    );
    
    result.push_str(&format!("💰 Current Price: ${:.4}\n\n", current_price));
    
    if let (Some(upper), Some(middle), Some(lower), Some(percent_b), Some(bandwidth)) = (
        bb_result.upper_band.last(),
        bb_result.middle_band.last(),
        bb_result.lower_band.last(),
        bb_result.percent_b.last(),
        bb_result.bandwidth.last(),
    ) {
        result.push_str(&format!("📈 Upper Band: ${:.4}\n", upper));
        result.push_str(&format!("📊 Middle Band (SMA): ${:.4}\n", middle));
        result.push_str(&format!("📉 Lower Band: ${:.4}\n", lower));
        result.push_str(&format!("📏 Bandwidth: {:.4}\n", bandwidth));
        result.push_str(&format!("📊 %B: {:.4}\n\n", percent_b));
        
        // Trading signals based on price position
        result.push_str("🎯 Trading Signals:\n");
        
        // Price position relative to bands
        if current_price > *upper {
            result.push_str("  • 🔴 OVERBOUGHT - Price above upper band, consider selling\n");
        } else if current_price < *lower {
            result.push_str("  • 🟢 OVERSOLD - Price below lower band, consider buying\n");
        } else if current_price > *middle {
            result.push_str("  • 📈 BULLISH - Price above middle band\n");
        } else {
            result.push_str("  • 📉 BEARISH - Price below middle band\n");
        }
        
        // %B interpretation
        if *percent_b > 1.0 {
            result.push_str("  • ⚠️ Price above upper band - Extreme overbought\n");
        } else if *percent_b < 0.0 {
            result.push_str("  • ⚠️ Price below lower band - Extreme oversold\n");
        } else if *percent_b > 0.8 {
            result.push_str("  • 🔴 High %B - Approaching overbought territory\n");
        } else if *percent_b < 0.2 {
            result.push_str("  • 🟢 Low %B - Approaching oversold territory\n");
        } else {
            result.push_str("  • ⚪ Neutral %B - Price within normal range\n");
        }
        
        // Bandwidth interpretation
        if *bandwidth > 0.1 {
            result.push_str("  • 📏 High Bandwidth - High volatility, expect breakout\n");
        } else if *bandwidth < 0.05 {
            result.push_str("  • 📏 Low Bandwidth - Low volatility, squeeze condition\n");
        } else {
            result.push_str("  • 📏 Normal Bandwidth - Average volatility\n");
        }
        
        // Bollinger Band squeeze detection
        if bb_result.bandwidth.len() >= 20 {
            let recent_bandwidth: Vec<f64> = bb_result.bandwidth.iter().rev().take(20).cloned().collect();
            let avg_bandwidth: f64 = recent_bandwidth.iter().sum::<f64>() / recent_bandwidth.len() as f64;
            
            if *bandwidth < avg_bandwidth * 0.7 {
                result.push_str("  • 🎯 BOLLINGER SQUEEZE - Volatility compression, breakout likely\n");
            }
        }
    }
    
    // Show recent Bollinger Bands values
    result.push_str("\n🕐 Recent Bollinger Bands Values:\n");
    let start_idx = if bb_result.upper_band.len() > 5 { 
        bb_result.upper_band.len() - 5 
    } else { 
        0 
    };
    
    for i in start_idx..bb_result.upper_band.len() {
        let idx = i - start_idx + 1;
        if let (Some(upper), Some(middle), Some(lower), Some(pb)) = (
            bb_result.upper_band.get(i),
            bb_result.middle_band.get(i),
            bb_result.lower_band.get(i),
            bb_result.percent_b.get(i),
        ) {
            result.push_str(&format!(
                "  {}. Upper: ${:.4} | Middle: ${:.4} | Lower: ${:.4} | %B: {:.3}\n",
                idx, upper, middle, lower, pb
            ));
        }
    }
    
    result
}

#[tool]
/// Calculate Bollinger Bands for volatility and mean reversion analysis
pub fn calculate_bollinger_bands_indicator(
    symbol: String,
    period: Option<String>,
    std_dev_multiplier: Option<String>,
) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20) as usize;
    let std_dev_multiplier = std_dev_multiplier
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(2.0);
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let bb_result = calculate_bollinger_bands(&closes, period, std_dev_multiplier);
                
                if let Some(current_price) = closes.last() {
                    format_bollinger_bands_analysis(&bb_result, *current_price, &symbol, period, std_dev_multiplier)
                } else {
                    format!("❌ No price data available for {}", symbol)
                }
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate Bollinger Bands with 24-hour data for comprehensive analysis
pub fn calculate_bollinger_bands_24h(
    symbol: String,
    period: Option<String>,
    std_dev_multiplier: Option<String>,
) -> String {
    let period = period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20) as usize;
    let std_dev_multiplier = std_dev_multiplier
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(2.0);
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 288).await { // 24h data
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let bb_result = calculate_bollinger_bands(&closes, period, std_dev_multiplier);
                
                if let Some(current_price) = closes.last() {
                    let mut result = format_bollinger_bands_analysis(&bb_result, *current_price, &symbol, period, std_dev_multiplier);
                    result.push_str("\n📊 Based on 24-hour data (288 candles)\n");
                    result
                } else {
                    format!("❌ No price data available for {}", symbol)
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

    #[test]
    fn test_std_dev_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let std_devs = calculate_std_dev(&values, 3);
        assert!(!std_devs.is_empty());
        // Standard deviation of [1,2,3] should be approximately 0.816
        assert!((std_devs[0] - 0.816).abs() < 0.01);
    }

    #[test]
    fn test_bollinger_bands_calculation() {
        let closes: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let bb_result = calculate_bollinger_bands(&closes, 20, 2.0);
        
        assert!(!bb_result.upper_band.is_empty());
        assert!(!bb_result.middle_band.is_empty());
        assert!(!bb_result.lower_band.is_empty());
        assert!(!bb_result.bandwidth.is_empty());
        assert!(!bb_result.percent_b.is_empty());
        
        // Upper band should be greater than middle band
        assert!(bb_result.upper_band[0] > bb_result.middle_band[0]);
        // Lower band should be less than middle band
        assert!(bb_result.lower_band[0] < bb_result.middle_band[0]);
    }
}