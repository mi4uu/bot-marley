
use crate::{binance::DataCollector, config::CONFIG};
// use botmarley::config::Config;
// use crate::logging::init_logger;
// use color_eyre::eyre::WrapErr;
// use color_eyre::Section;
use tracing::{info, error, warn};




pub async fn run_data_collector_init()->color_eyre::Result<()>{
        let config = (&*CONFIG).clone();

     // Create data collector
    let collector = match DataCollector::new(config) {
        Ok(collector) => collector,
        Err(e) => {
            error!("Failed to create data collector: {}", e);
            return Err(color_eyre::eyre::eyre!("Failed to create data collector: {}", e));
        }
    };

    // First, show existing data statistics
    info!("Checking existing data files...");
    match collector.get_data_stats().await {
        Ok(stats) => {
            if stats.is_empty() {
                info!("No existing data files found. Will collect data from the beginning.");
            } else {
                info!("=== Existing Data Statistics ===");
                for (symbol, (count, min_time, max_time)) in &stats {
                    let first_kline = min_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "N/A".to_string());
                    let last_kline = max_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "N/A".to_string());
                    
                    info!("  {}: {} records", symbol, count);
                    info!("    First kline: {}", first_kline);
                    info!("    Last kline:  {}", last_kline);
                    info!("");
                }
            }
        }
        Err(e) => {
            warn!("Failed to get existing data statistics: {}", e);
        }
    }


    Ok(())
}


pub async fn run_data_collector_collect()->color_eyre::Result<()>{
        let config = (&*CONFIG).clone();

     // Create data collector
    let collector = match DataCollector::new(config) {
        Ok(collector) => collector,
        Err(e) => {
            error!("Failed to create data collector: {}", e);
            return Err(color_eyre::eyre::eyre!("Failed to create data collector: {}", e));
        }
    };


    // Collect new data for all configured pairs
    info!("Starting data collection for all configured pairs...");
    match collector.collect_all_data().await {
        Ok(()) => {
            info!("Data collection completed successfully!");
          let _klines = collector.get_klines_for_symbol("BTCUSDC".into()).await?;
            // Show updated statistics
            match collector.get_data_stats().await {
                Ok(stats) => {
                    info!("=== Updated Data Statistics ===");
                    for (symbol, (count, min_time, max_time)) in stats {
                        let first_kline = min_time
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "N/A".to_string());
                        let last_kline = max_time
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "N/A".to_string());
                        
                        info!("  {}: {} records", symbol, count);
                        info!("    First kline: {}", first_kline);
                        info!("    Last kline:  {}", last_kline);
                        info!("");
                    }
                }
                Err(e) => error!("Failed to get updated statistics: {}", e),
            }
        }
        Err(e) => {
            error!("Data collection failed: {}", e);
            return Err(color_eyre::eyre::eyre!("Data collection failed: {}", e));
        }
    }
    Ok(())
}