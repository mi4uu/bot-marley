use std::time::Duration;
use botmarley::bot::Bot;
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with JSON file logging (hourly rotation)
    let file_appender = rolling::hourly("logs", "botmarley.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .json()
                .with_current_span(false)
                .with_span_list(true)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .compact()
        )
        .with(EnvFilter::from_default_env().add_directive("botmarley=info".parse()?))
        .init();

    info!("🚀 BotMarley - Crypto Trading Bot");
    dotenv::dotenv()?;
    
    let config = botmarley::config::Config::load();
    info!("📊 Config loaded:");
    info!("  - Max turns: {}", config.bot_max_turns);
    info!("  - Trading pairs: {:?}", config.pairs());
    info!("  - Model: {}", config.openai_model);
    
    // Create bot instance
    
    let test_symbols = vec!["BTCUSDT", "ETHUSDT"];
    let test_symbols = config.pairs();
    let mut bot = Bot::new(config).await?
        .add_system_message();
    
    info!("📊 Trading state loaded:");
    info!("  - Total previous runs: {}", bot.get_total_runs());

    loop{
        // Increment run counter at the start of each loop
        bot.increment_run_counter();
        info!("🔄 Starting run #{}", bot.get_total_runs());
        
        for symbol in test_symbols.clone() {
            println!("\n{}", "=".repeat(60));
            info!("🎯 Starting analysis for {}", symbol);
            println!("{}", "=".repeat(60));
            
            // Run the analysis loop
            match bot.run_analysis_loop(symbol.as_str()).await {
                Ok(result) => {
                    if result.turns_used == 0 && result.final_response.starts_with("Skipped:") {
                        info!("⏭️ Analysis Skipped for {} - {}", symbol, result.final_response);
                    } else {
                        info!("\n📈 Analysis Complete for {}!", symbol);
                        info!("  - Turns used: {}/{}", result.turns_used, bot.get_max_turns());
                        
                        match result.decision {
                            Some(decision) => {
                                info!("  - Decision: {:?}", decision);
                            }
                            None => {
                                info!("  - No final decision made within turn limit");
                            }
                        }
                        
                        info!("  - Final response length: {} chars", result.final_response.len());
                    }
                }
                Err(e) => {
                    error!("❌ Error analyzing {}: {}", symbol, e);
                }
            }
            
            // Reset for next symbol
            bot.reset_conversation();
            

        }
        
        info!("\n✅ All analyses completed! wait 1 minute");
    tokio::time::sleep(Duration::from_mins(1)).await;
    }
    Ok(())
}
