use std::time::Duration;
use botmarley::bot::Bot;
use botmarley::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    dotenv::dotenv().ok();
    let config = Config::load();
    
    // Create a new bot instance with system message
    let mut bot = Bot::new(config).await?
        .add_system_message();
    
    // Analyze a single symbol
    let symbol = "BTCUSDT";
    println!("🎯 Analyzing {}", symbol);
    
    match bot.run_analysis_loop(symbol).await {
        Ok(result) => {
            println!("✅ Analysis completed!");
            println!("Turns used: {}/{}", result.turns_used, bot.get_max_turns());
            
            if let Some(decision) = result.decision {
                println!("Decision: {:?}", decision);
            } else {
                println!("No decision made within turn limit");
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
    
    Ok(())
}