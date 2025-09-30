use std::time::Duration;
use botmarley::bot::Bot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 BotMarley - Crypto Trading Bot");
    dotenv::dotenv()?;
    
    let config = botmarley::config::Config::load();
    println!("📊 Config loaded:");
    println!("  - Max turns: {}", config.bot_max_turns);
    println!("  - Trading pairs: {:?}", config.pairs());
    println!("  - Model: {}", config.openai_model);
    
    // Create bot instance
    let mut bot = Bot::new(config).await?
        .add_system_message();
    
    // Test symbols to analyze
    let test_symbols = vec!["BTCUSDT", "ETHUSDT"];
    
    for symbol in test_symbols {
        println!("\n{}", "=".repeat(60));
        println!("🎯 Starting analysis for {}", symbol);
        println!("{}", "=".repeat(60));
        
        // Run the analysis loop
        match bot.run_analysis_loop(symbol).await {
            Ok(result) => {
                println!("\n📈 Analysis Complete for {}!", symbol);
                println!("  - Turns used: {}/{}", result.turns_used, bot.get_max_turns());
                
                match result.decision {
                    Some(decision) => {
                        println!("  - Decision: {:?}", decision);
                    }
                    None => {
                        println!("  - No final decision made within turn limit");
                    }
                }
                
                println!("  - Final response length: {} chars", result.final_response.len());
            }
            Err(e) => {
                println!("❌ Error analyzing {}: {}", symbol, e);
            }
        }
        
        // Reset for next symbol
        bot.reset_conversation();
        
        // Wait between analyses
        println!("\n⏳ Waiting 5 seconds before next analysis...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    
    println!("\n✅ All analyses completed!");
    Ok(())
}
