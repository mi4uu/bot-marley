# BotMarley - Easy-to-Use Crypto Trading Bot

## Overview

BotMarley is a crypto trading bot that uses AI to analyze market data and make trading decisions. The bot is designed to be easy to use and run in a loop with configurable turn limits and context preservation.

## Key Features

- **Turn Management**: AI can use up to `bot_max_turns` (configurable) to analyze and make decisions
- **Context Preservation**: Each turn includes information about remaining turns and previous context
- **Decision Tracking**: Automatically detects when AI makes final trading decisions (BUY/SELL/HOLD)
- **Easy Loop Integration**: Simple API for running multiple analyses
- **Conversation Reset**: Clean state management between different symbol analyses

## Configuration

Set these environment variables in your `.env` file:

```env
# AI Configuration
OPENAI_BASE_URL=http://localhost:1234/v1
OPENAI_API_KEY=your_api_key
OPENAI_MODEL=openai/gpt-oss-20b

# Trading Configuration
BOT_MAX_TURNS=5
ALLOWED_PAIRS=BTC_USDT,ETH_USDT
MAX_TRADE_VALUE=100
MAX_ACTIVE_ORDERS=3

# Binance API (if using real trading)
BINANCE_API_KEY=your_binance_key
BINANCE_SECRET_KEY=your_binance_secret
```

## Basic Usage

```rust
use botmarley::bot::Bot;
use botmarley::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load();
    
    // Create bot instance
    let mut bot = Bot::new(config).await?
        .add_system_message();
    
    // Analyze a symbol
    let result = bot.run_analysis_loop("BTCUSDT").await?;
    
    // Check the result
    match result.decision {
        Some(decision) => println!("Decision: {:?}", decision),
        None => println!("No decision made"),
    }
    
    Ok(())
}
```

## Advanced Usage - Multiple Symbols

```rust
let symbols = vec!["BTCUSDT", "ETHUSDT", "ADAUSDT"];

for symbol in symbols {
    println!("Analyzing {}", symbol);
    
    match bot.run_analysis_loop(symbol).await {
        Ok(result) => {
            println!("Turns used: {}/{}", result.turns_used, bot.get_max_turns());
            
            if let Some(decision) = result.decision {
                match decision {
                    BotDecision::Buy { pair, amount, confidence } => {
                        println!("🟢 BUY {} amount: {} confidence: {}%", pair, amount, confidence);
                    }
                    BotDecision::Sell { pair, amount, confidence } => {
                        println!("🔴 SELL {} amount: {} confidence: {}%", pair, amount, confidence);
                    }
                    BotDecision::Hold { pair, confidence } => {
                        println!("🟡 HOLD {} confidence: {}%", pair, confidence);
                    }
                }
            }
        }
        Err(e) => println!("Error analyzing {}: {}", symbol, e),
    }
    
    // Reset conversation for next symbol
    bot.reset_conversation();
}
```

## Bot Behavior

### Turn Management
- The bot starts each analysis with turn 1/max_turns
- Each turn, the AI receives context about remaining turns
- AI can take multiple turns to gather data and analyze before making a decision
- If max turns are reached without a decision, the bot forces a final decision

### Decision Making
The AI will:
1. Start by gathering market data using the `get_price` tool
2. Analyze the data step-by-step
3. When ready, make a final decision by calling one of:
   - `buy(pair, amount, confidence)` 
   - `sell(pair, amount, confidence)`
   - `hold(pair, confidence)`

### Context Preservation
- Each turn includes information about current turn number and remaining turns
- Previous conversation history is maintained throughout the analysis
- Context is reset between different symbol analyses

## API Reference

### Bot Struct

```rust
impl Bot {
    // Create new bot instance
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>>
    
    // Add system message with trading instructions
    pub fn add_system_message(self) -> Self
    
    // Add custom user message
    pub fn add_user_message(self, msg: String) -> Self
    
    // Run analysis loop for a symbol
    pub async fn run_analysis_loop(&mut self, symbol: &str) -> Result<BotResult, Box<dyn std::error::Error>>
    
    // Reset conversation state
    pub fn reset_conversation(&mut self)
    
    // Get conversation history
    pub fn get_conversation_history(&self) -> &Vec<Message>
    
    // Get current turn number
    pub fn get_current_turn(&self) -> usize
    
    // Get max turns from config
    pub fn get_max_turns(&self) -> usize
}
```

### BotResult

```rust
pub struct BotResult {
    pub decision: Option<BotDecision>,
    pub turns_used: usize,
    pub final_response: String,
    pub conversation_history: Vec<Message>,
}
```

### BotDecision

```rust
pub enum BotDecision {
    Buy { pair: String, amount: f64, confidence: usize },
    Sell { pair: String, amount: f64, confidence: usize },
    Hold { pair: String, confidence: usize },
}
```

## Running the Example

```bash
# Set up environment
cp .env.example .env
# Edit .env with your configuration

# Run the main example
cargo run

# Run the simple usage example
cargo run --example simple_bot_usage
```

## Tips

1. **Turn Limits**: Set `BOT_MAX_TURNS` based on your model's capabilities and desired analysis depth
2. **Model Selection**: Use models that support function calling for best results
3. **Error Handling**: Always handle potential errors from the analysis loop
4. **Rate Limiting**: Add delays between analyses to respect API rate limits
5. **Logging**: The bot provides detailed console output for monitoring analysis progress