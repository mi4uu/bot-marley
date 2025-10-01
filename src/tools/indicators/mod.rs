pub mod rsi;
pub mod moving_averages;
pub mod macd;
pub mod bollinger_bands;
pub mod atr;
pub mod stochastic;
pub mod volume_indicators;

// Common types and utilities for indicators
use crate::tools::get_prices::Kline;

#[derive(Debug, Clone)]
pub struct IndicatorResult {
    pub name: String,
    pub values: Vec<f64>,
    pub signals: Vec<String>,
    pub summary: String,
}

impl IndicatorResult {
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: Vec::new(),
            signals: Vec::new(),
            summary: String::new(),
        }
    }

    pub fn format_for_display(&self) -> String {
        let mut result = format!("📊 {} Analysis:\n", self.name);
        
        if !self.summary.is_empty() {
            result.push_str(&format!("📈 Summary: {}\n", self.summary));
        }
        
        if !self.values.is_empty() {
            if let Some(latest) = self.values.last() {
                result.push_str(&format!("🔢 Current Value: {:.4}\n", latest));
            }
        }
        
        if !self.signals.is_empty() {
            result.push_str("🎯 Signals:\n");
            for signal in &self.signals {
                result.push_str(&format!("  • {}\n", signal));
            }
        }
        
        result
    }
}

// Convert Kline data to close prices
pub fn klines_to_closes(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.close).collect()
}

// Convert Kline data to volumes
pub fn klines_to_volumes(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.volume).collect()
}

// Generate trading signals based on indicator values
pub fn generate_signals(indicator_name: &str, values: &[f64], current_price: f64) -> Vec<String> {
    let mut signals = Vec::new();
    
    if values.is_empty() {
        return signals;
    }
    
    let latest = match values.last() {
        Some(value) => value,
        None => return vec!["❌ Error: No values available for signal generation".to_string()],
    };
    
    match indicator_name.to_lowercase().as_str() {
        "rsi" => {
            if *latest > 70.0 {
                signals.push("🔴 OVERBOUGHT - Consider selling".to_string());
            } else if *latest < 30.0 {
                signals.push("🟢 OVERSOLD - Consider buying".to_string());
            } else if *latest > 50.0 {
                signals.push("📈 Bullish momentum".to_string());
            } else {
                signals.push("📉 Bearish momentum".to_string());
            }
        }
        "macd" => {
            if values.len() >= 2 {
                let prev = values[values.len() - 2];
                if *latest > 0.0 && prev <= 0.0 {
                    signals.push("🟢 BULLISH CROSSOVER - Buy signal".to_string());
                } else if *latest < 0.0 && prev >= 0.0 {
                    signals.push("🔴 BEARISH CROSSOVER - Sell signal".to_string());
                }
            }
        }
        "stochastic_k" => {
            if *latest > 80.0 {
                signals.push("🔴 OVERBOUGHT - Consider selling".to_string());
            } else if *latest < 20.0 {
                signals.push("🟢 OVERSOLD - Consider buying".to_string());
            }
        }
        _ => {}
    }
    
    signals
}