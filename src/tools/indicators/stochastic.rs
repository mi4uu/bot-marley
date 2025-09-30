use mono_ai_macros::tool;
use crate::tools::get_prices::{get_klines_for_indicators, Kline};

#[derive(Debug, Clone)]
pub struct StochasticResult {
    pub k_values: Vec<f64>,
    pub d_values: Vec<f64>,
    pub j_values: Vec<f64>,
}

/// Calculate Stochastic Oscillator (KDJ)
pub fn calculate_stochastic(klines: &[Kline], k_period: usize, d_period: usize) -> StochasticResult {
    if klines.len() < k_period {
        return StochasticResult {
            k_values: Vec::new(),
            d_values: Vec::new(),
            j_values: Vec::new(),
        };
    }

    let mut k_values = Vec::new();
    
    // Calculate %K values
    for i in k_period - 1..klines.len() {
        let period_slice = &klines[i - k_period + 1..=i];
        
        let highest_high = period_slice.iter().map(|k| k.high).fold(0.0, f64::max);
        let lowest_low = period_slice.iter().map(|k| k.low).fold(f64::INFINITY, f64::min);
        let current_close = klines[i].close;
        
        let k_value = if highest_high != lowest_low {
            ((current_close - lowest_low) / (highest_high - lowest_low)) * 100.0
        } else {
            50.0 // Neutral when no range
        };
        
        k_values.push(k_value);
    }
    
    // Calculate %D values (SMA of %K)
    let mut d_values = Vec::new();
    for i in d_period - 1..k_values.len() {
        let sum: f64 = k_values[i - d_period + 1..=i].iter().sum();
        d_values.push(sum / d_period as f64);
    }
    
    // Calculate %J values (3*%K - 2*%D)
    let mut j_values = Vec::new();
    let start_idx = if k_values.len() > d_values.len() {
        k_values.len() - d_values.len()
    } else {
        0
    };
    
    for i in 0..d_values.len() {
        let k_idx = start_idx + i;
        if k_idx < k_values.len() {
            let j_value = 3.0 * k_values[k_idx] - 2.0 * d_values[i];
            j_values.push(j_value);
        }
    }
    
    StochasticResult {
        k_values,
        d_values,
        j_values,
    }
}

pub fn format_stochastic_analysis(
    stoch_result: &StochasticResult,
    symbol: &str,
    k_period: usize,
    d_period: usize,
) -> String {
    let mut result = format!(
        "📊 Stochastic Oscillator KDJ({},{}) Analysis for {}:\n\n",
        k_period, d_period, symbol
    );
    
    if let (Some(k_val), Some(d_val), Some(j_val)) = (
        stoch_result.k_values.last(),
        stoch_result.d_values.last(),
        stoch_result.j_values.last(),
    ) {
        result.push_str(&format!("📈 %K Value: {:.2}\n", k_val));
        result.push_str(&format!("📊 %D Value: {:.2}\n", d_val));
        result.push_str(&format!("⚡ %J Value: {:.2}\n\n", j_val));
        
        // Overbought/Oversold analysis
        result.push_str("🎯 Market Conditions:\n");
        
        if *k_val > 80.0 && *d_val > 80.0 {
            result.push_str("  • 🔴 OVERBOUGHT - Both %K and %D above 80, consider selling\n");
        } else if *k_val < 20.0 && *d_val < 20.0 {
            result.push_str("  • 🟢 OVERSOLD - Both %K and %D below 20, consider buying\n");
        } else if *k_val > 80.0 {
            result.push_str("  • 🟡 %K OVERBOUGHT - Fast line showing selling pressure\n");
        } else if *k_val < 20.0 {
            result.push_str("  • 🟡 %K OVERSOLD - Fast line showing buying opportunity\n");
        } else {
            result.push_str("  • ⚪ NEUTRAL - No extreme overbought/oversold conditions\n");
        }
        
        // KD crossover signals
        result.push_str("\n📊 Trading Signals:\n");
        
        if *k_val > *d_val {
            result.push_str("  • 📈 BULLISH - %K above %D (fast line above slow line)\n");
        } else {
            result.push_str("  • 📉 BEARISH - %K below %D (fast line below slow line)\n");
        }
        
        // Check for recent crossovers
        if stoch_result.k_values.len() >= 2 && stoch_result.d_values.len() >= 2 {
            let prev_k = stoch_result.k_values[stoch_result.k_values.len() - 2];
            let prev_d = stoch_result.d_values[stoch_result.d_values.len() - 2];
            
            if *k_val > *d_val && prev_k <= prev_d {
                result.push_str("  • 🚀 GOLDEN CROSS - %K crossed above %D (BUY SIGNAL)\n");
            } else if *k_val < *d_val && prev_k >= prev_d {
                result.push_str("  • ⚠️ DEATH CROSS - %K crossed below %D (SELL SIGNAL)\n");
            }
        }
        
        // J line analysis
        result.push_str("\n⚡ %J Line Analysis:\n");
        if *j_val > 100.0 {
            result.push_str("  • 🔥 EXTREMELY OVERBOUGHT - %J above 100, strong sell signal\n");
        } else if *j_val < 0.0 {
            result.push_str("  • 🔥 EXTREMELY OVERSOLD - %J below 0, strong buy signal\n");
        } else if *j_val > 80.0 {
            result.push_str("  • 🔴 OVERBOUGHT - %J indicates selling pressure\n");
        } else if *j_val < 20.0 {
            result.push_str("  • 🟢 OVERSOLD - %J indicates buying opportunity\n");
        } else {
            result.push_str("  • ⚪ NEUTRAL - %J in normal range\n");
        }
        
        // Momentum analysis
        if stoch_result.k_values.len() >= 3 {
            let recent_k: Vec<f64> = stoch_result.k_values.iter().rev().take(3).cloned().collect();
            if recent_k[0] > recent_k[1] && recent_k[1] > recent_k[2] {
                result.push_str("  • 📈 ACCELERATING BULLISH - %K trending upward\n");
            } else if recent_k[0] < recent_k[1] && recent_k[1] < recent_k[2] {
                result.push_str("  • 📉 ACCELERATING BEARISH - %K trending downward\n");
            }
        }
    }
    
    // Show recent KDJ values
    result.push_str("\n🕐 Recent KDJ Values:\n");
    let start_idx = if stoch_result.d_values.len() > 5 { 
        stoch_result.d_values.len() - 5 
    } else { 
        0 
    };
    
    for i in start_idx..stoch_result.d_values.len() {
        let idx = i - start_idx + 1;
        let k_idx = if stoch_result.k_values.len() > stoch_result.d_values.len() {
            i + (stoch_result.k_values.len() - stoch_result.d_values.len())
        } else {
            i
        };
        
        if let (Some(k_val), Some(d_val), Some(j_val)) = (
            stoch_result.k_values.get(k_idx),
            stoch_result.d_values.get(i),
            stoch_result.j_values.get(i),
        ) {
            let signal = if *k_val > 80.0 { "🔴" } else if *k_val < 20.0 { "🟢" } else { "⚪" };
            result.push_str(&format!(
                "  {}. K: {:.1} | D: {:.1} | J: {:.1} {}\n",
                idx, k_val, d_val, j_val, signal
            ));
        }
    }
    
    result
}

#[tool]
/// Calculate Stochastic Oscillator (KDJ) for momentum analysis
pub fn calculate_stochastic_indicator(
    symbol: String,
    k_period: Option<String>,
    d_period: Option<String>,
) -> String {
    let k_period = k_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(14) as usize;
    let d_period = d_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(3) as usize;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let stoch_result = calculate_stochastic(&klines, k_period, d_period);
                format_stochastic_analysis(&stoch_result, &symbol, k_period, d_period)
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate Stochastic with 24-hour data for comprehensive momentum analysis
pub fn calculate_stochastic_24h(
    symbol: String,
    k_period: Option<String>,
    d_period: Option<String>,
) -> String {
    let k_period = k_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(14) as usize;
    let d_period = d_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(3) as usize;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 288).await { // 24h data
            Ok(klines) => {
                let stoch_result = calculate_stochastic(&klines, k_period, d_period);
                let mut result = format_stochastic_analysis(&stoch_result, &symbol, k_period, d_period);
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
    fn test_stochastic_calculation() {
        let klines = vec![
            Kline { high: 110.0, low: 100.0, close: 105.0, ..Default::default() },
            Kline { high: 115.0, low: 105.0, close: 110.0, ..Default::default() },
            Kline { high: 120.0, low: 110.0, close: 115.0, ..Default::default() },
            Kline { high: 118.0, low: 112.0, close: 114.0, ..Default::default() },
            Kline { high: 125.0, low: 115.0, close: 120.0, ..Default::default() },
        ];

        let stoch_result = calculate_stochastic(&klines, 3, 2);
        
        assert!(!stoch_result.k_values.is_empty());
        assert!(!stoch_result.d_values.is_empty());
        assert!(!stoch_result.j_values.is_empty());
        
        // K values should be between 0 and 100
        for k_val in &stoch_result.k_values {
            assert!(*k_val >= 0.0 && *k_val <= 100.0);
        }
    }

    #[test]
    fn test_stochastic_edge_cases() {
        // Test with identical high/low (no range)
        let klines = vec![
            Kline { high: 100.0, low: 100.0, close: 100.0, ..Default::default() },
            Kline { high: 100.0, low: 100.0, close: 100.0, ..Default::default() },
            Kline { high: 100.0, low: 100.0, close: 100.0, ..Default::default() },
        ];

        let stoch_result = calculate_stochastic(&klines, 3, 2);
        
        // Should handle zero range gracefully
        if !stoch_result.k_values.is_empty() {
            assert_eq!(stoch_result.k_values[0], 50.0);
        }
    }
}