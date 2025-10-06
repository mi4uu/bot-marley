
use std::sync::Arc;

use crate::binance::DataCollector;
use crate::config::{self, CONFIG, Config};
use crate::logging::init_logger;
use crate::symbol::Symbol;
use color_eyre::eyre::{Ok, WrapErr};
use color_eyre::Section;
use financial_indicators::macd::MACD;
use financial_indicators::mfi::money_flow_index;
use financial_indicators::rsi::relative_strength_index;
use financial_indicators::kdj::KDJ;
use financial_indicators::ma::simple_moving_average;
use financial_indicators::ema::exponential_moving_average;
use financial_indicators::atr::average_true_range;
use financial_indicators::bollinger::bollinger_bands;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use crate::binance::data_collector::KlineData;


#[derive(Debug,Clone,PartialEq,Serialize, Deserialize)]

pub struct KlinesOHLC{
    pub time: Vec<i64>,

    // time: Vec<chrono::NaiveDateTime>,
    pub open: Vec<f64>,
   pub close: Vec<f64>,
  pub  high: Vec<f64>,
 pub   low: Vec<f64>,
 pub   volume: Vec<f64>,
 pub count:usize

}

#[derive(Debug,Clone)]

pub struct Klines{
    symbol:Symbol,

    klines: Option<Vec<KlineData>>,
    ohcl: Option<KlinesOHLC>,
    collector:DataCollector

}
impl Klines{
    #[instrument]

    pub fn new(symbol:Symbol)->Klines{
        let config=(&*CONFIG).clone();

    let collector = DataCollector::new((&config).clone()).expect("unable to get collector");
        Klines { symbol,  collector, ohcl:None,klines:None }
    }
#[instrument(skip(self),level="debug")]
    pub async fn get_klinedata(&mut self)-> color_eyre::Result<Vec<KlineData>>{
       if let Some(klines_data)=&self.klines{
            return Ok(klines_data.clone());
        }
    let klines: Vec<crate::binance::data_collector::KlineData> = self.collector.get_klines_for_symbol(self.symbol.to_string()).await?;
        self.klines=Some(klines.clone());
        Ok(klines)
    }
    #[instrument(skip(self))]

    pub async fn get_ohlc(&mut self)-> color_eyre::Result<KlinesOHLC>{
        if let Some(ohcl_data)=&self.ohcl{
            return Ok(ohcl_data.clone());
        }
    let klines=self.get_klinedata().await?;
    let mut time: Vec<i64>=Vec::new();
    let mut close:Vec<f64>=Vec::new();
    let mut low:Vec<f64>=Vec::new();
    let mut high:Vec<f64>=Vec::new();
    let mut volume:Vec<f64>=Vec::new();
    let mut open:Vec<f64>=Vec::new();
    for kline in klines.iter() {
      
        close.push(kline.close);
        low.push(kline.low);
        high.push(kline.high);
        open.push(kline.open);
        volume.push(kline.volume);
        time.push(kline.open_time);
    }
    let ohcl=KlinesOHLC{
        time,
        close,
        open,
        low,
        high,
        volume,
        count: klines.iter().count()
    };
    self.ohcl=Some(ohcl.clone());


    Ok(ohcl)
    }




}






async fn get_klines_with_metadata(symbol:String, config:Config) -> color_eyre::Result<Vec<KlinesWithMetadata>> {


    // Create data collector
    let collector = DataCollector::new(Arc::new(config))?;

   
    let klines: Vec<crate::binance::data_collector::KlineData> = collector.get_klines_for_symbol(symbol).await?;

    let mut close:Vec<f64>=Vec::new();
    let mut low:Vec<f64>=Vec::new();
    let mut high:Vec<f64>=Vec::new();
    let mut volume:Vec<f64>=Vec::new();
    let mut open:Vec<f64>=Vec::new();
    

    // Collect more kline data to ensure indicators have enough data points
    // We need at least 50+ data points for proper indicator calculations
    let data_points = 100.min(klines.len()); // Use up to 100 klines or all available
    
    for (i, kline) in klines.iter().take(data_points).enumerate() {
        if i < 35 {
            println!("Kline {}: {:#?}", i + 1, kline);
        }
        close.push(kline.close);
        low.push(kline.low);
        high.push(kline.high);
        open.push(kline.open);
        volume.push(kline.volume);
    }

    println!("Collected {} data points for indicator calculations", data_points);

    // Calculate indicators after collecting all data
    println!("Starting indicator calculations with {} data points", close.len());
    println!("Sample close prices: {:?}", &close[0..5.min(close.len())]);
    
    let macd_values = MACD::new(&close, 12, 26, 9);
    println!("MACD values length: {}", macd_values.len());
    if !macd_values.is_empty() {
        println!("Sample MACD: {:?}", &macd_values[0..3.min(macd_values.len())]);
    }
    
    let period: usize = 14;
    let mfi = money_flow_index(&high, &low, &close, &volume, period);
    println!("MFI values length: {}", mfi.len());
    if !mfi.is_empty() {
        println!("Sample MFI: {:?}", &mfi[0..3.min(mfi.len())]);
    }
    
    let rsi = relative_strength_index(&close, period);
    println!("RSI values length: {}", rsi.len());
    if !rsi.is_empty() {
        println!("Sample RSI: {:?}", &rsi[0..3.min(rsi.len())]);
    }
    
    let period = 3;
    let kdj_values = KDJ::new(&high, &low, &close, period);
    println!("KDJ values length: {}", kdj_values.len());
    if !kdj_values.is_empty() {
        println!("Sample KDJ: {:?}", &kdj_values[0..3.min(kdj_values.len())]);
    }
    
    let sma = simple_moving_average(&close, period);
    println!("SMA values length: {}", sma.len());
    if !sma.is_empty() {
        println!("Sample SMA: {:?}", &sma[0..3.min(sma.len())]);
    }
    
    let _ema = exponential_moving_average(&close, period);
    let _atr = average_true_range(&high, &low, &close, period);

    let period = 20;
    let k = 2.0;
    let bb = bollinger_bands(&close, period, k);
    println!("Bollinger Bands - Upper: {}, Middle: {}, Lower: {}", bb.0.len(), bb.1.len(), bb.2.len());
    if !bb.0.is_empty() {
        println!("Sample BB Upper: {:?}", &bb.0[0..3.min(bb.0.len())]);
    }

    // Create KlinesWithIndicators for each kline
    let mut klines_with_indicators: Vec<KlinesWithIndicators> = Vec::new();
    
    for (i, kline) in klines.iter().enumerate() {
        let kwi = KlinesWithIndicators {
            open: kline.open,
            high: kline.high,
            low: kline.low,
            close: kline.close,
            volume: kline.volume,
            macd: if i < macd_values.len() {
                // Create a new MACD instance with the same values
                MACD {
                    macd: macd_values[i].macd,
                    signal: macd_values[i].signal,
                    histogram: macd_values[i].histogram
                }
            } else {
                // Create a default MACD with zero values
                MACD { macd: 0.0, signal: 0.0, histogram: 0.0 }
            },
            mfi: if i < mfi.len() {
                mfi[i].unwrap_or(0.0)
            } else {
                0.0
            },
            rsi: if i < rsi.len() {
                rsi[i].unwrap_or(0.0)
            } else {
                0.0
            },
            kdj: if i < kdj_values.len() {
                // Create a new KDJ instance with the same values
                KDJ {
                    k: kdj_values[i].k,
                    d: kdj_values[i].d,
                    j: kdj_values[i].j
                }
            } else {
                // Create a default KDJ with zero values
                KDJ { k: 0.0, d: 0.0, j: 0.0 }
            },
            bollinger_bands: vec![
                if i < bb.0.len() { bb.0[i].unwrap_or(0.0) } else { 0.0 }, // upper band
                if i < bb.1.len() { bb.1[i].unwrap_or(0.0) } else { 0.0 }, // middle band
                if i < bb.2.len() { bb.2[i].unwrap_or(0.0) } else { 0.0 }, // lower band
            ],
        };
        
        klines_with_indicators.push(kwi);
    }

    dbg!(&klines_with_indicators);
    
    // Map KlinesWithIndicators to KlinesWithMetadata
    let klines_with_metadata: Vec<KlinesWithMetadata> = klines_with_indicators
        .into_iter()
        .enumerate()
        .map(|(index, kwi)| {
            let current_price = kwi.close;
            let target_increase = current_price * 1.02; // 2% increase threshold
            let target_decrease = current_price * 0.95; // 5% decrease threshold
            
            // Calculate predictions based on maximum/minimum prices reached within time windows
            // 30m = 6 klines ahead (5m intervals), 1h = 12 klines, 2h = 24 klines
            
            // Check if price increases by 2% at any point within 30 minutes (6 klines)
            let price_will_increase_2percent_in_30m = klines
                .iter()
                .skip(index + 1)
                .take(6)
                .any(|future_kline| future_kline.high >= target_increase);
                
            // Check if price increases by 2% at any point within 1 hour (12 klines)
            let price_will_increase_2percent_in_1h = klines
                .iter()
                .skip(index + 1)
                .take(12)
                .any(|future_kline| future_kline.high >= target_increase);
                
            // Check if price increases by 2% at any point within 2 hours (24 klines)
            let price_will_increase_2percent_in_2h = klines
                .iter()
                .skip(index + 1)
                .take(24)
                .any(|future_kline| future_kline.high >= target_increase);
                            // Check if price increases by 2% at any point within 2 hours (24 klines)
            let price_will_increase_2percent_in_8h = klines
                .iter()
                .skip(index + 1)
                .take(96)
                .any(|future_kline| future_kline.high >= target_increase);
                
            // Check if price drops by 5% at any point within 2 hours (24 klines)
            let price_will_drop_over_5percent_in_2h = klines
                .iter()
                .skip(index + 1)
                .take(24)
                .any(|future_kline| future_kline.low <= target_decrease);
            
            KlinesWithMetadata {
                kline: kwi,
                price_will_increase_2percent_in_30m,
                price_will_increase_2percent_in_1h,
                price_will_increase_2percent_in_2h,
                price_will_increase_2percent_in_8h,
                price_will_drop_over_5percent_in_2h,
            }
        })
        .collect();

    dbg!(&klines_with_metadata);

    Ok(klines_with_metadata)
}

pub struct KlinesWithIndicators {
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    volume: f64,
    macd: MACD,
    mfi: f64,
    rsi: f64,
    kdj: KDJ,
    bollinger_bands: Vec<f64>,
}

impl std::fmt::Debug for KlinesWithIndicators {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlinesWithIndicators")
            .field("open", &self.open)
            .field("close", &self.close)
            .field("high", &self.high)
            .field("low", &self.low)
            .field("volume", &self.volume)
            .field("macd", &self.macd)
            .field("mfi", &self.mfi)
            .field("rsi", &self.rsi)
            .field("kdj", &self.kdj)
            .field("bollinger_bands", &self.bollinger_bands)
            .finish()
    }
}
pub struct KlinesWithMetadata {
    kline: KlinesWithIndicators,
    price_will_increase_2percent_in_30m: bool,
    price_will_increase_2percent_in_1h: bool,
    price_will_increase_2percent_in_2h: bool,
    price_will_increase_2percent_in_8h:bool,
    price_will_drop_over_5percent_in_2h: bool,
}

impl std::fmt::Debug for KlinesWithMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlinesWithMetadata")
            .field("kline", &self.kline)
            .field("price_will_increase_2percent_in_30m", &self.price_will_increase_2percent_in_30m)
            .field("price_will_increase_2percent_in_1h", &self.price_will_increase_2percent_in_1h)
            .field("price_will_increase_2percent_in_2h", &self.price_will_increase_2percent_in_2h)
            .field("price_will_drop_over_5percent_in_2h", &self.price_will_drop_over_5percent_in_2h)
            .finish()
    }
}