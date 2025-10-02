# BotMarley - AI-Powered Crypto Trading Bot

<div align="center">
  <img src="static/logo.png" alt="BotMarley Logo" width="600"/>
  
  [![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
  [![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)
  [![Version](https://img.shields.io/badge/version-0.1.0-green.svg?style=for-the-badge)](Cargo.toml)
</div>

## 🚀 Overview

BotMarley is an intelligent cryptocurrency trading bot built in Rust that leverages AI to analyze market data and make informed trading decisions. The bot features a sophisticated turn-based analysis system, real-time portfolio tracking, and a web-based dashboard for monitoring performance.

## ✨ Key Features

- **🤖 AI-Powered Analysis**: Uses advanced AI models to analyze market trends and make trading decisions
- **🔄 Turn-Based Decision Making**: Configurable turn limits with context preservation across analysis cycles
- **📊 Real-Time Portfolio Tracking**: Automatic portfolio snapshots and performance monitoring
- **🌐 Web Dashboard**: Built-in web interface for monitoring bot activity and portfolio status
- **📰 News Integration**: Fetches and analyzes latest crypto news for informed decision making
- **🔧 Flexible Configuration**: Easy-to-configure trading pairs, limits, and AI model settings
- **📈 Technical Indicators**: Built-in support for RSI, MACD, Bollinger Bands, and more
- **🛡️ Risk Management**: Configurable trade limits and position sizing
- **📝 Comprehensive Logging**: Detailed JSON logging with hourly rotation

## 🏗️ Architecture

The bot is built with a modular architecture:

- **Core Bot Engine**: AI-driven analysis and decision making
- **Binance Integration**: Real-time market data and trade execution
- **Portfolio Tracker**: Asset tracking and performance monitoring
- **Web Server**: Dashboard and API endpoints
- **Technical Indicators**: Market analysis tools
- **News Fetcher**: Crypto news integration
- **Persistence Layer**: State management and data storage

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (2024 edition)
- Binance API credentials (for live trading)
- AI model API access (OpenAI compatible)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/botmarley.git
   cd botmarley
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   ```
   
   Edit `.env` with your configuration:
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
   WEB_UI_PORT=3000
   
   # Binance API
   BINANCE_API_KEY=your_binance_key
   BINANCE_SECRET_KEY=your_binance_secret
   ```

3. **Build and run**
   ```bash
   cargo build --release
   cargo run
   ```

4. **Access the web dashboard**
   Open your browser to `http://localhost:3000`

## 📖 Usage

### Basic Bot Usage

```rust
use botmarley::bot::Bot;
use botmarley::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load();
    
    // Create bot instance
    let mut bot = Bot::new(config).await?;
    
    // Analyze a symbol
    let result = bot.run_analysis_loop("BTCUSDT", "".into()).await?;
    
    // Check the result
    match result.decision {
        Some(decision) => println!("Decision: {:?}", decision),
        None => println!("No decision made"),
    }
    
    Ok(())
}
```

### Running the Web Server Only

```bash
cargo run --bin webserver
```

### Multiple Symbol Analysis

The bot can analyze multiple trading pairs in sequence:

```rust
let symbols = vec!["BTCUSDT", "ETHUSDT", "ADAUSDT"];

for symbol in symbols {
    match bot.run_analysis_loop(&symbol, "".into()).await {
        Ok(result) => {
            println!("✅ Analysis complete for {}", symbol);
            if let Some(decision) = result.decision {
                println!("Decision: {:?}", decision);
            }
        }
        Err(e) => println!("❌ Error analyzing {}: {}", symbol, e),
    }
    
    // Reset for next symbol
    bot.reset_conversation();
}
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENAI_BASE_URL` | AI model API endpoint | - |
| `OPENAI_API_KEY` | API key for AI model | - |
| `OPENAI_MODEL` | Model name to use | - |
| `BOT_MAX_TURNS` | Maximum analysis turns per symbol | 5 |
| `ALLOWED_PAIRS` | Comma-separated trading pairs | - |
| `MAX_TRADE_VALUE` | Maximum trade value in USDT | 100 |
| `MAX_ACTIVE_ORDERS` | Maximum concurrent orders | 3 |
| `WEB_UI_PORT` | Web dashboard port | 3000 |
| `BINANCE_API_KEY` | Binance API key | - |
| `BINANCE_SECRET_KEY` | Binance secret key | - |

### Trading Pairs

Configure trading pairs in the `ALLOWED_PAIRS` environment variable:
```env
ALLOWED_PAIRS=BTC_USDT,ETH_USDT,ADA_USDT,DOT_USDT
```

## 📊 Technical Indicators

BotMarley includes a comprehensive set of technical indicators:

- **Moving Averages**: SMA, EMA, WMA
- **Momentum**: RSI, Stochastic Oscillator
- **Trend**: MACD, Bollinger Bands
- **Volatility**: ATR (Average True Range)
- **Volume**: Volume-based indicators

## 🌐 Web Dashboard

The built-in web dashboard provides:

- Real-time portfolio overview
- Trading history and performance
- Bot status and configuration
- Market data visualization
- Log monitoring

Access at `http://localhost:3000` (or your configured port)

## 📁 Project Structure

```
botmarley/
├── src/
│   ├── bin/
│   │   └── webserver.rs          # Standalone web server
│   ├── tools/
│   │   ├── indicators/           # Technical indicators
│   │   ├── binance_trade.rs      # Trading tools
│   │   └── get_prices.rs         # Price fetching
│   ├── utils/
│   │   └── cryptonews.rs         # News integration
│   ├── bot.rs                    # Core bot logic
│   ├── config.rs                 # Configuration management
│   ├── portfolio.rs              # Portfolio tracking
│   ├── web_server.rs             # Web interface
│   └── main.rs                   # Main application
├── static/
│   ├── index.html                # Web dashboard
│   └── logo.png                  # Bot logo
├── data/                         # Data storage
├── logs/                         # Log files
└── crates/
    └── mono-ai/                  # AI integration library
```

## 🔍 Monitoring and Logging

BotMarley provides comprehensive logging:

- **JSON Logs**: Structured logging with hourly rotation
- **Console Output**: Real-time status updates
- **Web Dashboard**: Visual monitoring interface
- **Portfolio Snapshots**: Automatic performance tracking

Log files are stored in the `logs/` directory with timestamps.

## 🛡️ Risk Management

The bot includes several risk management features:

- **Position Sizing**: Configurable maximum trade values
- **Order Limits**: Maximum concurrent active orders
- **Turn Limits**: Prevents infinite analysis loops
- **Error Handling**: Graceful error recovery
- **Portfolio Tracking**: Real-time risk monitoring

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

**This software is for educational and research purposes only. Cryptocurrency trading involves substantial risk of loss. The authors are not responsible for any financial losses incurred through the use of this software. Always do your own research and never invest more than you can afford to lose.**

## 🆘 Support

- 📖 Check the [BOT_USAGE.md](BOT_USAGE.md) for detailed usage instructions
- 🐛 Report issues on GitHub
- 💬 Join our community discussions

---

<div align="center">
  <p>Made with ❤️ and ☕ by the BotMarley team</p>
  <p>🚀 Happy Trading! 🚀</p>
</div>