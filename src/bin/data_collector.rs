use botmarley::binance::DataCollector;
use tracing::{info, error};
use color_eyre::eyre::WrapErr;
use color_eyre::Section;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
color_eyre::install()?;
    dotenv::dotenv()
    .wrap_err("Failed to load .env file")
    .with_suggestion(|| "Make sure .env file exists and is readable")?;
    let config = (&*botmarley::config::CONFIG).clone();

    
    // Create data collector
    let collector = DataCollector::new(config)?;

    // Collect data for all configured pairs
    match collector.collect_all_data().await {
        Ok(()) => {
            info!("Data collection completed successfully");
            
            // Show statistics
            match collector.get_data_stats().await {
                Ok(stats) => {
                    info!("Data collection statistics:");
                    for (symbol, (count, min_time, max_time)) in stats {
                        let min_str = min_time
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "N/A".to_string());
                        let max_str = max_time
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "N/A".to_string());
                        
                        info!("  {}: {} records, from {} to {}", 
                              symbol, count, min_str, max_str);
                    }
                }
                Err(e) => error!("Failed to get statistics: {}", e),
            }
        }
        Err(e) => {
            error!("Data collection failed: {}", e);
            return Err(e);
        }
    }

    info!("Data collector finished");
    Ok(())
}