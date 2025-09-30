use botmarley::persistence::{PersistenceManager, TradingState, TradingDecision};
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 BotMarley Persistence System Demo");
    println!("=====================================\n");

    // Initialize persistence manager
    let persistence_manager = PersistenceManager::new("demo_trading_state.json");
    
    // Load existing state or create new one
    let mut state = persistence_manager.load_state();
    
    println!("📊 Current State:");
    println!("  - Total runs: {}", state.total_runs);
    println!("  - Symbols tracked: {}", state.symbols.len());
    
    // Simulate some trading decisions
    let decisions = vec![
        TradingDecision {
            symbol: "BTCUSDT".to_string(),
            action: "BUY".to_string(),
            amount: Some(100.0),
            confidence: 85,
            explanation: "Strong bullish momentum with RSI oversold recovery".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(45000.0),
        },
        TradingDecision {
            symbol: "ETHUSDT".to_string(),
            action: "HOLD".to_string(),
            amount: None,
            confidence: 70,
            explanation: "Consolidation phase, waiting for breakout".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(3000.0),
        },
        TradingDecision {
            symbol: "BTCUSDT".to_string(),
            action: "SELL".to_string(),
            amount: Some(50.0),
            confidence: 90,
            explanation: "Target reached, taking profits at resistance level".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(46000.0),
        },
    ];

    println!("\n🔄 Adding new trading decisions...");
    for decision in decisions {
        println!("  - {} {} ({}%): {}", 
            decision.symbol, 
            decision.action, 
            decision.confidence,
            decision.explanation
        );
        state.add_decision(decision);
    }

    // Increment run counter
    state.increment_runs();

    // Save updated state
    persistence_manager.save_state(&state)?;
    println!("\n💾 State saved successfully!");

    // Display trading history summaries
    println!("\n📈 Trading History Summaries:");
    println!("{}", "=".repeat(50));
    
    for symbol in ["BTCUSDT", "ETHUSDT", "ADAUSDT"] {
        println!("{}", state.generate_context_summary(symbol));
    }

    println!("\n✅ Demo completed! Check 'demo_trading_state.json' for the saved state.");
    
    Ok(())
}