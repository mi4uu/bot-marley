
use std::sync::Arc;

// use botmarley::{binance::DataCollector, config::CONFIG};
// use botmarley::config::Config;
use botmarley::{bot::{indicators::macd::Macd, klines}, logging::init_logger, symbol::{self, Symbol}};
use color_eyre::eyre::Ok;
use tokio::sync::Mutex;
use tracing::{info, instrument};
// use color_eyre::eyre::WrapErr;
// use color_eyre::Section;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;
    // let config = (&*CONFIG).clone();
    init_logger();

    tracing::info!("🚀  Starting BotMarley");
    // botmarley::binance::main_collector_runner::run_data_collector_init().await?;
    // botmarley::binance::main_collector_runner::run_data_collector_collect().await?;

    let symbol:Symbol="BTCUSDC".into();
 let _btcbot=bot_run(symbol).await?;


    tracing::info!("🏁 Stoping BotMarley. Bye");
    Ok(())
}


#[instrument]
async fn bot_run(symbol:Symbol)->color_eyre::Result<()>{

//    let mut klines=botmarley::bot::klines::Klines::new(symbol);
   let  klines=Arc::new(Mutex::new(botmarley::bot::klines::Klines::new(symbol)));
    // let klinedata=klines.lock().await.get_klinedata().await?;

    let ohcl_data=klines.lock().await.get_ohlc().await?;

    let ohcl_count=ohcl_data.count;
    tracing::info!( ohcl_count=ohcl_count);

    let mut macd=Macd{
        klines,

    };
    // Use the new aligned MACD calculation
    let aligned_macd = macd.calculate_aligned().await?;
    info!(
        macd_len=aligned_macd.macd_values.len(),
        offset=aligned_macd.offset,
        total_klines=aligned_macd.total_klines,
        "MACD alignment info"
    );
  
       if let Some(macd_value) = aligned_macd.get_macd_for_kline(30){
                info!(
                    kline_index=30,
                    // timestamp=ma.open_time,
                    // close=kline_data.close,
                    macd_line=macd_value.macd,
                    signal_line=macd_value.signal,
                    histogram=macd_value.histogram
                );
    }

    // Demonstrate proper alignment - iterate through klines and assign MACD values
    // for (i, kline_data) in klinedata.iter().enumerate() {
    //     if let Some(macd_value) = aligned_macd.get_macd_for_kline(i) {
    //         // This kline has a corresponding MACD value
    //         if i < aligned_macd.offset + 5 { // Only log first few MACD values for brevity
    //             info!(
    //                 kline_index=i,
    //                 timestamp=kline_data.open_time,
    //                 close=kline_data.close,
    //                 macd_line=macd_value.macd,
    //                 signal_line=macd_value.signal,
    //                 histogram=macd_value.histogram
    //             );
    //         }
    //     } else {
    //         // This kline is in the warm-up period
    //         if i < 5 { // Only log first few for brevity
    //             info!(
    //                 kline_index=i,
    //                 timestamp=kline_data.open_time,
    //                 close=kline_data.close,
    //                 macd="None (warm-up period)"
    //             );
    //         }
    //     }
    // }

    Ok(())
}