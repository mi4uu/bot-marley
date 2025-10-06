use std::sync::{Arc};

use color_eyre::eyre::Ok;
use financial_indicators::macd::MACD;
use tokio::sync::Mutex;

use crate::bot::klines::Klines;

pub struct Macd {
  pub  klines:Arc<Mutex<Klines>>,
  //  pub  period: usize

}

/// Represents a MACD calculation result aligned with kline data
#[derive(Debug)]
pub struct AlignedMacdResult {
    pub macd_values: Vec<MACD>,
    pub offset: usize,
    pub total_klines: usize,
}

impl AlignedMacdResult {
    /// Get MACD value for a specific kline index, returns None if in warm-up period
    pub fn get_macd_for_kline(&self, kline_index: usize) -> Option<&MACD> {
        if kline_index < self.offset {
            None // Warm-up period
        } else {
            let macd_index = kline_index - self.offset;
            self.macd_values.get(macd_index)
        }
    }
    
    /// Check if a kline index has a corresponding MACD value
    pub fn has_macd_for_kline(&self, kline_index: usize) -> bool {
        kline_index >= self.offset && (kline_index - self.offset) < self.macd_values.len()
    }
}

impl Macd{
    pub async fn calculate(&mut self)->color_eyre::Result<Vec<MACD>>{
      let close=self.klines.lock().await.get_ohlc().await?.close;
      let macd_values = MACD::new(&close, 12, 26, 9);
      Ok(macd_values)
    }
    
    /// Calculate MACD with proper alignment information
    pub async fn calculate_aligned(&mut self) -> color_eyre::Result<AlignedMacdResult> {
        let ohlc = self.klines.lock().await.get_ohlc().await?;
        let close = ohlc.close;
        let total_klines = ohlc.count;
        
        let macd_values = MACD::new(&close, 12, 26, 9);
        let offset = total_klines - macd_values.len();
        
        Ok(AlignedMacdResult {
            macd_values,
            offset,
            total_klines,
        })
    }
}