use mono_ai_macros::tool;
use crate::tools::get_prices::get_klines_for_indicators;
use crate::tools::indicators::moving_averages::calculate_ema;

#[derive(Debug, Clone)]
pub struct MacdResult {
    pub macd_line: Vec<f64>,
    pub signal_line: Vec<f64>,
    pub histogram: Vec<f64>,
}

/// Calculate MACD (Moving Average Convergence Divergence)
pub fn calculate_macd(closes: &[f64], fast_period: usize, slow_period: usize, signal_period: usize) -> MacdResult {
    let fast_ema = calculate_ema(closes, fast_period);
    let slow_ema = calculate_ema(closes, slow_period);
    
    // Calculate MACD line (fast EMA - slow EMA)
    let mut macd_line = Vec::new();
    let start_idx = if fast_ema.len() > slow_ema.len() { 
        fast_ema.len() - slow_ema.len() 
    } else { 
        0 
    };
    
    for i in start_idx..fast_ema.len().min(slow_ema.len()) {
        macd_line.push(fast_ema[i] - slow_ema[i - start_idx]);
    }
    
    // Calculate signal line (EMA of MACD line)
    let signal_line = calculate_ema(&macd_line, signal_period);
    
    // Calculate histogram (MACD - Signal)
    let mut histogram = Vec::new();
    let hist_start = if macd_line.len() > signal_line.len() {
        macd_line.len() - signal_line.len()
    } else {
        0
    };
    
    for i in hist_start..macd_line.len().min(signal_line.len()) {
        histogram.push(macd_line[i] - signal_line[i - hist_start]);
    }
    
    MacdResult {
        macd_line,
        signal_line,
        histogram,
    }
}

pub fn format_macd_analysis(
    macd_result: &MacdResult,
    symbol: &str,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> String {
    let mut result = format!(
        "📊 MACD({},{},{}) Analysis for {}:\n\n",
        fast_period, slow_period, signal_period, symbol
    );
    
    if let (Some(macd), Some(signal), Some(histogram)) = (
        macd_result.macd_line.last(),
        macd_result.signal_line.last(),
        macd_result.histogram.last(),
    ) {
        result.push_str(&format!("📈 MACD Line: {:.6}\n", macd));
        result.push_str(&format!("📊 Signal Line: {:.6}\n", signal));
        result.push_str(&format!("📊 Histogram: {:.6}\n\n", histogram));
        
        // MACD signals
        result.push_str("🎯 Trading Signals:\n");
        
        // Bullish/Bearish based on MACD line position
        if *macd > 0.0 {
            result.push_str("  • 📈 MACD above zero - Bullish momentum\n");
        } else {
            result.push_str("  • 📉 MACD below zero - Bearish momentum\n");
        }
        
        // Signal line crossover
        if *macd > *signal {
            result.push_str("  • 🟢 MACD above Signal - Buy signal\n");
        } else {
            result.push_str("  • 🔴 MACD below Signal - Sell signal\n");
        }
        
        // Histogram analysis
        if *histogram > 0.0 {
            result.push_str("  • 📊 Positive Histogram - Momentum increasing\n");
        } else {
            result.push_str("  • 📊 Negative Histogram - Momentum decreasing\n");
        }
        
        // Check for crossovers in recent data
        if macd_result.macd_line.len() >= 2 && macd_result.signal_line.len() >= 2 {
            let prev_macd = macd_result.macd_line[macd_result.macd_line.len() - 2];
            let prev_signal = macd_result.signal_line[macd_result.signal_line.len() - 2];
            
            if *macd > *signal && prev_macd <= prev_signal {
                result.push_str("  • 🚀 BULLISH CROSSOVER - Strong buy signal!\n");
            } else if *macd < *signal && prev_macd >= prev_signal {
                result.push_str("  • ⚠️ BEARISH CROSSOVER - Strong sell signal!\n");
            }
        }
    }
    
    // Show recent MACD values
    result.push_str("\n🕐 Recent MACD Values:\n");
    let start_idx = if macd_result.macd_line.len() > 5 { 
        macd_result.macd_line.len() - 5 
    } else { 
        0 
    };
    
    for i in start_idx..macd_result.macd_line.len() {
        let idx = i - start_idx + 1;
        if let (Some(macd), Some(signal), Some(hist)) = (
            macd_result.macd_line.get(i),
            macd_result.signal_line.get(i.saturating_sub(start_idx)),
            macd_result.histogram.get(i.saturating_sub(start_idx)),
        ) {
            let trend = if *macd > *signal { "📈" } else { "📉" };
            result.push_str(&format!(
                "  {}. MACD: {:.6} | Signal: {:.6} | Hist: {:.6} {}\n",
                idx, macd, signal, hist, trend
            ));
        }
    }
    
    result
}

#[tool]
/// Calculate MACD (Moving Average Convergence Divergence) for technical analysis
pub fn calculate_macd_indicator(
    symbol: String,
    fast_period: Option<String>,
    slow_period: Option<String>,
    signal_period: Option<String>,
) -> String {
    let fast_period = fast_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(12) as usize;
    let slow_period = slow_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(26) as usize;
    let signal_period = signal_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(9) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 100).await {
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let macd_result = calculate_macd(&closes, fast_period, slow_period, signal_period);
                
                format_macd_analysis(&macd_result, &symbol, fast_period, slow_period, signal_period)
            }
            Err(e) => {
                format!("❌ Error fetching price data for {}: {}", symbol, e)
            }
        }
    })
}

#[tool]
/// Calculate MACD with 24-hour data for more comprehensive analysis
pub fn calculate_macd_24h(
    symbol: String,
    fast_period: Option<String>,
    slow_period: Option<String>,
    signal_period: Option<String>,
) -> String {
    let fast_period = fast_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(12) as usize;
    let slow_period = slow_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(26) as usize;
    let signal_period = signal_period
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(9) as usize;
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => return format!("❌ Failed to create async runtime: {}", e),
    };
    rt.block_on(async {
        match get_klines_for_indicators(&symbol, "5m", 288).await { // 24h data
            Ok(klines) => {
                let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let macd_result = calculate_macd(&closes, fast_period, slow_period, signal_period);
                
                let mut result = format_macd_analysis(&macd_result, &symbol, fast_period, slow_period, signal_period);
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

    #[test]
    fn test_macd_calculation() {
        let closes: Vec<f64> = (1..=50).map(|i| i as f64).collect();
        let macd_result = calculate_macd(&closes, 12, 26, 9);
        
        assert!(!macd_result.macd_line.is_empty());
        assert!(!macd_result.signal_line.is_empty());
        assert!(!macd_result.histogram.is_empty());
    }
}