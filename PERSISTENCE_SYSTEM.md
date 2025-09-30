# BotMarley Trading Decision Persistence System

## Overview

The persistence system enables BotMarley to remember and learn from previous trading decisions across bot runs. This system stores trading tool parameters, decision history, and provides context to the AI for improved decision-making.

## Key Features

### 🔄 Decision Persistence
- **Buy/Sell/Hold decisions** with full parameters (pair, amount, confidence, explanation)
- **Timestamps** for all decisions
- **Price at decision** tracking (when available)
- **Decision counters** per symbol (buy_count, sell_count, hold_count)

### 📊 Historical Context
- **Previous decision summaries** included in system prompts
- **Recent decision patterns** for each symbol
- **Confidence level tracking** over time
- **Explanation history** for learning from past reasoning

### 💾 File-Based Storage
- **JSON format** for human-readable persistence
- **Automatic backup** creation before updates
- **Error handling** with graceful fallbacks
- **Directory auto-creation** for storage paths

## Architecture

### Core Components

#### 1. TradingDecision
```rust
pub struct TradingDecision {
    pub symbol: String,           // Trading pair (e.g., "BTCUSDT")
    pub action: String,           // "BUY", "SELL", or "HOLD"
    pub amount: Option<f64>,      // Trade amount (None for HOLD)
    pub confidence: usize,        // Confidence percentage (0-100)
    pub explanation: String,      // AI's reasoning
    pub timestamp: DateTime<Utc>, // When decision was made
    pub price_at_decision: Option<f64>, // Market price at decision
}
```

#### 2. SymbolHistory
```rust
pub struct SymbolHistory {
    pub symbol: String,
    pub decisions: Vec<TradingDecision>,
    pub last_decision: Option<TradingDecision>,
    pub total_decisions: usize,
    pub buy_count: usize,
    pub sell_count: usize,
    pub hold_count: usize,
}
```

#### 3. TradingState
```rust
pub struct TradingState {
    pub symbols: HashMap<String, SymbolHistory>,
    pub last_updated: Option<DateTime<Utc>>,
    pub total_runs: usize,
}
```

#### 4. PersistenceManager
```rust
pub struct PersistenceManager {
    file_path: String,
}
```

## Integration with Bot

### System Prompt Enhancement
The bot's system message now includes guidance about using previous decision context:

```
Previous Decision Context:
• You will be provided with your previous trading decisions for each symbol.
• Consider your past decisions, their confidence levels, and explanations.
• Learn from previous patterns - if you consistently made certain decisions, analyze why.
• Avoid repeating the same mistakes or being overly conservative/aggressive based on past performance.
• Use historical context to improve decision quality, but don't be bound by past decisions if market conditions have changed.
```

### Context Injection
Before each analysis, the bot receives a trading history summary:

```
📊 TRADING HISTORY FOR BTCUSDT:
  • Total decisions: 2 (Buy: 1, Sell: 1, Hold: 0)
  • Last decision: SELL (Confidence: 90%) at 2025-09-30 21:09:43 UTC
  • Last explanation: Target reached, taking profits at resistance level
  • Price at last decision: $46000.0000
  • Recent decisions:
    - SELL (90%) on 09-30 21:09
    - BUY (85%) on 09-30 21:09
```

### Decision Capture
When trading tools ([`buy`](src/tools/binance_trade.rs:19), [`sell`](src/tools/binance_trade.rs:11), [`hold`](src/tools/binance_trade.rs:27)) are executed, the system automatically:

1. **Extracts parameters** from tool arguments
2. **Creates TradingDecision** record
3. **Updates symbol history** and counters
4. **Saves state** to persistent storage

## Usage Examples

### Loading Previous State
```rust
let persistence_manager = PersistenceManager::new("data/trading_state.json");
let trading_state = persistence_manager.load_state();
```

### Adding New Decision
```rust
let decision = TradingDecision {
    symbol: "BTCUSDT".to_string(),
    action: "BUY".to_string(),
    amount: Some(100.0),
    confidence: 85,
    explanation: "Strong bullish signal".to_string(),
    timestamp: Utc::now(),
    price_at_decision: Some(45000.0),
};

trading_state.add_decision(decision);
```

### Generating Context Summary
```rust
let context = trading_state.generate_context_summary("BTCUSDT");
// Returns formatted string with trading history
```

### Saving State
```rust
persistence_manager.save_state(&trading_state)?;
```

## File Structure

### Storage Location
- **Default path**: `data/trading_state.json`
- **Configurable** via PersistenceManager constructor
- **Auto-created directories** if they don't exist

### JSON Format
```json
{
  "symbols": {
    "BTCUSDT": {
      "symbol": "BTCUSDT",
      "decisions": [...],
      "last_decision": {...},
      "total_decisions": 2,
      "buy_count": 1,
      "sell_count": 1,
      "hold_count": 0
    }
  },
  "last_updated": "2025-09-30T21:09:43.309455Z",
  "total_runs": 1
}
```

## Benefits

### 🧠 Learning from History
- **Pattern recognition**: AI can identify successful/unsuccessful decision patterns
- **Confidence calibration**: Track confidence vs. actual outcomes
- **Strategy refinement**: Adjust approach based on historical performance

### 📈 Improved Decision Quality
- **Context awareness**: Decisions informed by previous market behavior
- **Consistency**: Avoid contradictory decisions without good reason
- **Risk management**: Learn from past losses and successes

### 🔍 Debugging & Analysis
- **Decision audit trail**: Complete history of all trading decisions
- **Performance tracking**: Analyze bot behavior over time
- **Strategy validation**: Verify if trading logic is working as expected

## Testing

### Unit Tests
Run the persistence system tests:
```bash
cargo test test_persistence_system
```

### Demo Application
Try the interactive demo:
```bash
cargo run --example persistence_demo
```

## Future Enhancements

### Potential Improvements
- **Performance metrics**: Track P&L for each decision
- **Market condition correlation**: Link decisions to market indicators
- **Strategy evolution**: Automatic strategy adjustment based on performance
- **Backup rotation**: Automatic cleanup of old backup files
- **Compression**: Compress historical data for large datasets

### Integration Opportunities
- **Real-time price tracking**: Capture actual prices at decision time
- **Outcome tracking**: Monitor if BUY/SELL decisions were profitable
- **Strategy backtesting**: Use historical decisions to validate strategies