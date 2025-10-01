use std::time::Duration;
use botmarley::bot::Bot;
use botmarley::logging::{CustomJsonFormatter, LocalTimeFileAppender};
use botmarley::web_server;
use tracing::{info, error, info_span, Instrument};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use color_eyre::eyre::{Result, WrapErr, eyre};
use color_eyre::Section;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;
    
    dotenv::dotenv()
        .wrap_err("Failed to load .env file")
        .with_suggestion(|| "Make sure .env file exists and is readable")?;
    let config = botmarley::config::Config::load();
    
    // Initialize tracing with custom JSON file logging (hourly rotation with local time)
    let file_appender = LocalTimeFileAppender::new("logs", "botmarley")
        .wrap_err("Failed to create log file appender")
        .with_suggestion(|| "Make sure the logs directory is writable")?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .event_format(CustomJsonFormatter)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .compact()
        )
        .with(EnvFilter::from_default_env().add_directive("botmarley=info".parse()
            .wrap_err("Failed to parse log filter directive")?))
        .init();

    info!("🚀 BotMarley - Crypto Trading Bot");
    
    info!("📊 Config loaded:");
    info!("  - Max turns: {}", config.bot_max_turns);
    info!("  - Trading pairs: {:?}", config.pairs());
    info!("  - Model: {}", config.openai_model);
    info!("  - Web UI Port: {}", config.web_ui_port);
    
    // Start web server in background
    let web_port = config.web_ui_port as u16;
    tokio::spawn(async move {
        if let Err(e) = web_server::start_web_server(web_port).await {
            error!("❌ Web server error: {:?}", e);
        }
    });
    
    // Create bot instance
    let test_symbols = config.pairs();
    let mut bot = match Bot::new(config).await {
        Ok(bot) => bot.add_system_message(),
        Err(e) => {
            return Err(eyre!("Failed to initialize trading bot: {}", e)
                .with_suggestion(|| "Check your API credentials and network connection"));
        }
    };
    
    info!("📊 Trading state loaded:");
    info!("  - Total previous runs: {}", bot.get_total_runs());

    loop {
        // Increment run counter at the start of each loop
        bot.increment_run_counter();
        let run_number = bot.get_total_runs();
        
        let _run_span = info_span!("trading_run", run = run_number).entered();
        info!("🔄 Starting run #{}", run_number);
        
        for symbol in test_symbols.clone() {
            println!("\n{}", "=".repeat(60));
            println!("{}", "=".repeat(60));
            
            let symbol_span = info_span!("symbol_analysis", symbol = symbol.as_str(), run = run_number);
            
            async {
                info!("🎯 Starting analysis for {}", symbol);
                
                // Run the analysis loop
                match bot.run_analysis_loop(symbol.as_str()).await {
                    Ok(result) => {
                        if result.turns_used == 0 && result.final_response.starts_with("Skipped:") {
                            info!("⏭️ Analysis Skipped for {} - {}", symbol, result.final_response);
                        } else {
                            info!("📈 Analysis Complete for {}!", symbol);
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
                        error!("❌ Error analyzing {}: {:?}", symbol, e);
                        // Continue with next symbol instead of crashing
                    }
                }
                
                // Reset for next symbol
                bot.reset_conversation();
            }.instrument(symbol_span).await;
        }
        
        info!("✅ All analyses completed! Waiting 1 minute...");
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
