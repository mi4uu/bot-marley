use botmarley::binance::DataCollector;
use botmarley::config::Config;
use botmarley::logging::init_logger;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug)]
struct MultiPairPortfolio {
    initial_value: f64,
    current_value: f64,
    cash_amount: f64,
    holdings: HashMap<String, f64>, // symbol -> amount
    transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
struct Transaction {
    timestamp: i64,
    pair: String,
    action: String, // "buy", "sell"
    price: f64,
    amount: f64,
    value: f64,
    portfolio_value_after: f64,
}

impl MultiPairPortfolio {
    fn new(initial_value: f64) -> Self {
        let mut portfolio = Self {
            initial_value,
            current_value: initial_value,
            cash_amount: initial_value,
            holdings: HashMap::new(),
            transactions: Vec::new(),
        };
        
        // Add some sample transactions for demonstration
        portfolio.transactions.push(Transaction {
            timestamp: 1696118400000, // 2023-10-01 00:00:00
            pair: "BTCUSDC".to_string(),
            action: "buy".to_string(),
            price: 63000.0,
            amount: 0.1,
            value: 6300.0,
            portfolio_value_after: 9700.0,
        });
        
        portfolio.transactions.push(Transaction {
            timestamp: 1696122000000, // 2023-10-01 01:00:00
            pair: "ETHUSDC".to_string(),
            action: "buy".to_string(),
            price: 2500.0,
            amount: 1.0,
            value: 2500.0,
            portfolio_value_after: 7200.0,
        });
        
        portfolio.transactions.push(Transaction {
            timestamp: 1696125600000, // 2023-10-01 02:00:00
            pair: "BTCUSDC".to_string(),
            action: "sell".to_string(),
            price: 63500.0,
            amount: 0.05,
            value: 3175.0,
            portfolio_value_after: 10375.0,
        });
        
        // Update holdings
        portfolio.holdings.insert("BTCUSDC".to_string(), 0.05);
        portfolio.holdings.insert("ETHUSDC".to_string(), 1.0);
        portfolio.cash_amount = 7200.0;
        portfolio.current_value = 10375.0;
        
        portfolio
    }
}

fn generate_multi_pair_user_message(
    all_klines: &HashMap<String, Vec<botmarley::binance::data_collector::KlineData>>, 
    index: usize,
    portfolio: &MultiPairPortfolio,
    allowed_pairs: &[String]
) -> String {
    let mut message = String::new();
    
    message.push_str("=== MULTI-PAIR TRADING ANALYSIS ===\n\n");
    
    // Add current portfolio information
    message.push_str("=== CURRENT PORTFOLIO ===\n");
    message.push_str(&format!("Total Portfolio Value: ${:.2}\n", portfolio.current_value));
    message.push_str(&format!("Available Cash: ${:.2}\n", portfolio.cash_amount));
    message.push_str("Holdings:\n");
    
    for (symbol, amount) in &portfolio.holdings {
        if *amount > 0.0 {
            message.push_str(&format!("  {}: {:.6}\n", symbol, amount));
        }
    }
    message.push_str("\n");
    
    // Add last 10 transactions
    message.push_str("=== LAST 10 TRANSACTIONS ===\n");
    let recent_transactions: Vec<_> = portfolio.transactions.iter().rev().take(10).collect();
    if recent_transactions.is_empty() {
        message.push_str("No previous transactions\n");
    } else {
        for transaction in recent_transactions.iter().rev() {
            let time_str = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(transaction.timestamp)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M:%S UTC");
            message.push_str(&format!(
                "  {} | {} {} | {:.6} @ ${:.2} | Value: ${:.2} | Portfolio: ${:.2}\n",
                time_str,
                transaction.action.to_uppercase(),
                transaction.pair,
                transaction.amount,
                transaction.price,
                transaction.value,
                transaction.portfolio_value_after
            ));
        }
    }
    message.push_str("\n");
    
    // Add market data for all allowed pairs (limited to first 3 for demo)
    message.push_str("=== MARKET DATA FOR ALL PAIRS ===\n");
    
    for (i, pair) in allowed_pairs.iter().take(3).enumerate() {
        if let Some(klines) = all_klines.get(pair) {
            if !klines.is_empty() && index < klines.len() {
                message.push_str(&format!("\n--- {} ---\n", pair));
                message.push_str(&generate_pair_analysis(klines, index));
            }
        }
        if i < 2 { // Add separator between pairs
            message.push_str("\n");
        }
    }
    
    message.push_str("\n=== TRADING INSTRUCTIONS ===\n");
    message.push_str("Based on the above market data, portfolio status, and transaction history:\n");
    message.push_str("1. Analyze all available trading pairs\n");
    message.push_str("2. Consider your current portfolio allocation\n");
    message.push_str("3. Review recent transaction patterns\n");
    message.push_str("4. Make a single trading decision (buy/sell/hold)\n");
    message.push_str("5. If buying/selling, specify the exact pair and amount\n");
    message.push_str("6. Consider risk management and diversification\n");
    message.push_str(&format!("\nAvailable pairs for trading: {}\n", allowed_pairs.join(", ")));
    
    message
}

fn generate_pair_analysis(klines: &[botmarley::binance::data_collector::KlineData], index: usize) -> String {
    if klines.is_empty() || index >= klines.len() {
        return "No kline data available".to_string();
    }

    let current = &klines[index];
    let mut analysis = String::new();

    analysis.push_str(&format!("Current Price: ${:.4}\n", current.close));
    analysis.push_str(&format!(
        "OHLC: O:${:.4} H:${:.4} L:${:.4} C:${:.4}\n",
        current.open, current.high, current.low, current.close
    ));
    analysis.push_str(&format!("Volume: {:.2}\n", current.volume));

    // Add recent price trend
    analysis.push_str("Recent 3 candles:\n");
    let recent_start = if index >= 2 { index - 2 } else { 0 };
    for i in recent_start..=index {
        let k = &klines[i];
        let change = if i > 0 {
            let prev_close = klines[i - 1].close;
            ((k.close - prev_close) / prev_close) * 100.0
        } else {
            0.0
        };
        
        analysis.push_str(&format!(
            "  {}: ${:.4} ({:+.2}%)\n",
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(k.open_time)
                .unwrap_or_default()
                .format("%H:%M"),
            k.close,
            change
        ));
    }

    analysis
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logger();

    let config = (&*botmarley::config::CONFIG).clone();
    let allowed_pairs = vec!["BTCUSDC".to_string(), "ETHUSDC".to_string(), "BNBUSDC".to_string()];
    
    info!("Generating sample user message for pairs: {:?}", allowed_pairs);

    // Initialize data collector
    let collector = DataCollector::new(config.clone())?;

    // Collect data for first 3 pairs only (for demo)
    let mut all_klines = HashMap::new();
    for pair in &allowed_pairs {
        info!("Loading sample data for {}", pair);
        let klines = collector.get_klines_for_symbol(pair.clone()).await?;
        if !klines.is_empty() {
            all_klines.insert(pair.clone(), klines);
        }
    }

    // Initialize sample portfolio
    let portfolio = MultiPairPortfolio::new(10000.0);

    // Generate user message (using index 50 for demonstration)
    let index = 50;
    let user_message = generate_multi_pair_user_message(&all_klines, index, &portfolio, &allowed_pairs);

    println!("\n=== EXAMPLE USER MESSAGE THAT WOULD BE SENT TO LLM ===\n");
    println!("{}", user_message);
    println!("\n=== END OF USER MESSAGE ===\n");

    Ok(())
}