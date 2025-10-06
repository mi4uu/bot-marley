
use botmarley::binance::DataCollector;
use botmarley::bot::system::get_system_message;
use botmarley::config::Config;
use botmarley::logging::init_logger;
use botmarley::utils::date_to_timestamp::date_string_to_timestamp;
use color_eyre::eyre::WrapErr;
use color_eyre::Section;
use financial_indicators::macd::MACD;
use financial_indicators::mfi::money_flow_index;
use financial_indicators::rsi::relative_strength_index;
use financial_indicators::ma::simple_moving_average;
use financial_indicators::ema::exponential_moving_average;
use financial_indicators::bollinger::bollinger_bands;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use chrono::{DateTime, Utc};
use tracing::{info, instrument};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct TradingDecision {
    action: String, // "buy", "sell", "hold"
    pair: String,   // Which pair to trade
    amount: f64,    // Amount to trade
    confidence: f64, // 0.0 to 1.0
    reasoning: String,
    thinking: Vec<Thought>,
    price_target: Option<f64>,
    stop_loss: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Thought {
    step: f64,  
    thought: String,
    require_next_step: bool,
    conclusion: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LLMRequest {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonSchema {
    name: String,
    strict: bool,
    schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct LLMResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug)]
struct DecisionResult {
    timestamp: i64,
    pair: String,
    price: f64,
    decision: TradingDecision,
    actual_price_30m: Option<f64>,
    actual_price_1h: Option<f64>,
    actual_price_2h: Option<f64>,
    was_correct_30m: Option<bool>,
    was_correct_1h: Option<bool>,
    was_correct_2h: Option<bool>,
    profit_loss_30m: Option<f64>,
    profit_loss_1h: Option<f64>,
    profit_loss_2h: Option<f64>,
}

#[derive(Debug)]
struct TestReport {
    // Test configuration
    pairs: Vec<String>,
    model: String,
    test_period_from: String,
    test_period_to: String,
    config: Config,
    
    // Basic statistics
    total_decisions: usize,
    buy_decisions: usize,
    sell_decisions: usize,
    hold_decisions: usize,
    
    // Per-pair statistics
    pair_stats: HashMap<String, PairStatistics>,
    
    // Overall accuracy metrics
    correct_predictions_30m: usize,
    correct_predictions_1h: usize,
    correct_predictions_2h: usize,
    wrong_predictions_30m: usize,
    wrong_predictions_1h: usize,
    wrong_predictions_2h: usize,
    
    // Financial metrics
    total_profit_loss_30m: f64,
    total_profit_loss_1h: f64,
    total_profit_loss_2h: f64,
    average_profit_per_decision_30m: f64,
    average_profit_per_decision_1h: f64,
    average_profit_per_decision_2h: f64,
    best_decision_profit_30m: f64,
    best_decision_profit_1h: f64,
    best_decision_profit_2h: f64,
    worst_decision_loss_30m: f64,
    worst_decision_loss_1h: f64,
    worst_decision_loss_2h: f64,
    
    // Accuracy percentages
    accuracy_30m: f64,
    accuracy_1h: f64,
    accuracy_2h: f64,
    buy_accuracy_30m: f64,
    buy_accuracy_1h: f64,
    buy_accuracy_2h: f64,
    sell_accuracy_30m: f64,
    sell_accuracy_1h: f64,
    sell_accuracy_2h: f64,
    
    // Portfolio simulation
    initial_portfolio_value: f64,
    final_portfolio_value_30m: f64,
    final_portfolio_value_1h: f64,
    final_portfolio_value_2h: f64,
    portfolio_return_30m: f64,
    portfolio_return_1h: f64,
    portfolio_return_2h: f64,
    
    results: Vec<DecisionResult>,
}

#[derive(Debug)]
struct PairStatistics {
    pair: String,
    decisions: usize,
    buy_decisions: usize,
    sell_decisions: usize,
    hold_decisions: usize,
    accuracy_30m: f64,
    accuracy_1h: f64,
    accuracy_2h: f64,
    profit_loss_30m: f64,
    profit_loss_1h: f64,
    profit_loss_2h: f64,
}

#[derive(Debug, Clone)]
struct MultiPairPortfolio {
    initial_value: f64,
    current_value: f64,
    cash_amount: f64,
    holdings: HashMap<String, f64>, // pair -> amount
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

async fn make_llm_request(config: &Config, system_message: &str, user_message: &str) -> color_eyre::Result<TradingDecision> {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "thinking": {  
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "step": {
                            "type": "integer"
                        },
                        "thought": {
                            "type": "string"
                        },
                        "require_next_step":{
                            "type": "boolean"
                        },
                        "conclusion": {
                            "type": "string"
                        }
                    },
                    "required": ["step", "thought", "require_next_step","conclusion" ],
                    "additionalProperties": false
                }
            },
            "reasoning": {
                "type": "string"
            },
            "action": {
                "type": "string",
                "enum": ["buy", "sell", "hold"]
            },
            "pair": {
                "type": "string"
            },
            "amount": {
                "type": "number",
                "minimum": 0.0
            },
            "confidence": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0
            },
            "price_target": {
                "type": "number"
            },
            "stop_loss": {
                "type": "number"
            },
        },
        "required": ["thinking", "reasoning", "action", "pair", "amount", "confidence" ],  
        "additionalProperties": false
    });

    let request = LLMRequest {
        model: config.openai_model.clone(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_message.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ],
        response_format: ResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: JsonSchema {
                name: "trading_decision".to_string(),
                strict: true,
                schema,
            },
        },
    };

    let response = client
        .post(&format!("{}/v1/chat/completions", config.openai_base_url))
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .wrap_err("Failed to send request to LLM")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(color_eyre::eyre::eyre!("LLM API error: {}", error_text));
    }

    let llm_response: LLMResponse = response
        .json()
        .await
        .wrap_err("Failed to parse LLM response")?;

    let content = &llm_response.choices[0].message.content;
    let decision: TradingDecision = serde_json::from_str(content)
        .wrap_err("Failed to parse trading decision JSON")?;
    
    info!("DECISION: {}", serde_json::to_string_pretty(&decision).unwrap());
    Ok(decision)
}

fn generate_multi_pair_user_message(
    all_klines: &HashMap<String, Vec<botmarley::binance::data_collector::KlineData>>, 
    index: usize, 
    portfolio: &MultiPairPortfolio,
    allowed_pairs: &[String]
) -> String {
    let mut message = String::new();
    
    message.push_str("=== MULTI-PAIR TRADING ANALYSIS ===\n\n");
    
    // Add current portfolio status
    message.push_str("=== CURRENT PORTFOLIO ===\n");
    message.push_str(&format!("Total Portfolio Value: ${:.2}\n", portfolio.current_value));
    message.push_str(&format!("Available Cash: ${:.2}\n", portfolio.cash_amount));
    message.push_str("Holdings:\n");
    
    if portfolio.holdings.is_empty() {
        message.push_str("  No current holdings\n");
    } else {
        for (pair, amount) in &portfolio.holdings {
            if *amount > 0.0 {
                message.push_str(&format!("  {}: {:.6}\n", pair, amount));
            }
        }
    }
    message.push_str("\n");
    
    // Add last 10 transactions
    message.push_str("=== LAST 10 TRANSACTIONS ===\n");
    if portfolio.transactions.is_empty() {
        message.push_str("No previous transactions\n");
    } else {
        let recent_transactions: Vec<_> = portfolio.transactions.iter().rev().take(10).collect();
        for transaction in recent_transactions.iter().rev() {
            let time_str = DateTime::<Utc>::from_timestamp_millis(transaction.timestamp)
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
    
    // Add market data for all allowed pairs
    message.push_str("=== MARKET DATA FOR ALL PAIRS ===\n");
    
    for pair in allowed_pairs {
        if let Some(klines) = all_klines.get(pair) {
            if !klines.is_empty() && index < klines.len() {
                message.push_str(&format!("\n--- {} ---\n", pair));
                message.push_str(&generate_pair_analysis(klines, index));
            }
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
    let start_idx = if index >= 20 { index - 20 } else { 0 };
    let context_klines = &klines[start_idx..=index];

    // Calculate technical indicators
    let closes: Vec<f64> = context_klines.iter().map(|k| k.close).collect();
    let highs: Vec<f64> = context_klines.iter().map(|k| k.high).collect();
    let lows: Vec<f64> = context_klines.iter().map(|k| k.low).collect();
    let volumes: Vec<f64> = context_klines.iter().map(|k| k.volume).collect();

    let mut analysis = String::new();

    analysis.push_str(&format!("Current Price: ${:.4}\n", current.close));
    analysis.push_str(&format!(
        "OHLC: O:${:.4} H:${:.4} L:${:.4} C:${:.4}\n",
        current.open, current.high, current.low, current.close
    ));
    analysis.push_str(&format!("Volume: {:.2}\n", current.volume));

    // Add technical indicators if we have enough data
    if closes.len() >= 14 {
        let rsi = relative_strength_index(&closes, 14);
        if let Some(current_rsi) = rsi.last().and_then(|r| *r) {
            analysis.push_str(&format!("RSI(14): {:.2}\n", current_rsi));
        }

        let mfi = money_flow_index(&highs, &lows, &closes, &volumes, 14);
        if let Some(current_mfi) = mfi.last().and_then(|m| *m) {
            analysis.push_str(&format!("MFI(14): {:.2}\n", current_mfi));
        }
    }

    // Moving averages
    if closes.len() >= 20 {
        let sma_20 = simple_moving_average(&closes, 20);
        if let Some(current_sma_20) = sma_20.last().and_then(|s| *s) {
            analysis.push_str(&format!("SMA(20): ${:.4}\n", current_sma_20));
        }

        let ema_20 = exponential_moving_average(&closes, 20);
        if let Some(current_ema_20) = ema_20.last().and_then(|e| *e) {
            analysis.push_str(&format!("EMA(20): ${:.4}\n", current_ema_20));
        }

        let bb = bollinger_bands(&closes, 20, 2.0);
        if let (Some(upper), Some(middle), Some(lower)) = (
            bb.0.last().and_then(|u| *u),
            bb.1.last().and_then(|m| *m),
            bb.2.last().and_then(|l| *l),
        ) {
            analysis.push_str(&format!(
                "BB: U:${:.4} M:${:.4} L:${:.4}\n",
                upper, middle, lower
            ));
        }
    }

    // MACD
    if closes.len() >= 26 {
        let macd_values = MACD::new(&closes, 12, 26, 9);
        if let Some(current_macd) = macd_values.last() {
            analysis.push_str(&format!(
                "MACD: {:.4} SIG: {:.4} HIST: {:.4}\n",
                current_macd.macd, current_macd.signal, current_macd.histogram
            ));
        }
    }

    // Recent price trend
    analysis.push_str("Recent 5 candles:\n");
    let recent_start = if index >= 4 { index - 4 } else { 0 };
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
            DateTime::<Utc>::from_timestamp_millis(k.open_time)
                .unwrap_or_default()
                .format("%H:%M"),
            k.close,
            change
        ));
    }

    analysis
}

impl MultiPairPortfolio {
    fn new(initial_value: f64) -> Self {
        Self {
            initial_value,
            current_value: initial_value,
            cash_amount: initial_value,
            holdings: HashMap::new(),
            transactions: Vec::new(),
        }
    }

    fn execute_trade(&mut self, decision: &TradingDecision, current_prices: &HashMap<String, f64>, timestamp: i64) {
        if decision.action == "hold" {
            return;
        }

        let price = match current_prices.get(&decision.pair) {
            Some(p) => *p,
            None => return,
        };

        match decision.action.as_str() {
            "buy" => {
                let trade_value = decision.amount * price;
                if self.cash_amount >= trade_value {
                    self.cash_amount -= trade_value;
                    *self.holdings.entry(decision.pair.clone()).or_insert(0.0) += decision.amount;
                    
                    let transaction = Transaction {
                        timestamp,
                        pair: decision.pair.clone(),
                        action: "buy".to_string(),
                        price,
                        amount: decision.amount,
                        value: trade_value,
                        portfolio_value_after: self.calculate_portfolio_value(current_prices),
                    };
                    self.transactions.push(transaction);
                }
            }
            "sell" => {
                let current_holding = *self.holdings.get(&decision.pair).unwrap_or(&0.0);
                let sell_amount = decision.amount.min(current_holding);
                
                if sell_amount > 0.0 {
                    let trade_value = sell_amount * price;
                    self.cash_amount += trade_value;
                    *self.holdings.entry(decision.pair.clone()).or_insert(0.0) -= sell_amount;
                    
                    let transaction = Transaction {
                        timestamp,
                        pair: decision.pair.clone(),
                        action: "sell".to_string(),
                        price,
                        amount: sell_amount,
                        value: trade_value,
                        portfolio_value_after: self.calculate_portfolio_value(current_prices),
                    };
                    self.transactions.push(transaction);
                }
            }
            _ => {}
        }
        
        self.current_value = self.calculate_portfolio_value(current_prices);
    }

    fn calculate_portfolio_value(&self, current_prices: &HashMap<String, f64>) -> f64 {
        let mut total_value = self.cash_amount;
        
        for (pair, amount) in &self.holdings {
            if let Some(price) = current_prices.get(pair) {
                total_value += amount * price;
            }
        }
        
        total_value
    }
}

async fn filter_klines_by_date_range(
    klines: Vec<botmarley::binance::data_collector::KlineData>,
    start_date: &str,
    end_date: &str,
) -> color_eyre::Result<Vec<botmarley::binance::data_collector::KlineData>> {
    let start_timestamp = date_string_to_timestamp(start_date)? * 1000;
    let end_timestamp = date_string_to_timestamp(end_date)? * 1000;

    let filtered: Vec<_> = klines
        .into_iter()
        .filter(|kline| {
            kline.open_time >= start_timestamp && kline.open_time <= end_timestamp
        })
        .collect();

    println!("Filtered {} klines for date range {} to {}", filtered.len(), start_date, end_date);
    Ok(filtered)
}

fn evaluate_decision(
    decision: &TradingDecision,
    current_price: f64,
    best_price: f64,
) -> (bool, f64, bool) {
    match decision.action.as_str() {
        "buy" => {
            let is_correct = best_price > current_price;
            let profit_loss = ((best_price - current_price) / current_price) * 100.0;
            let is_fatal = profit_loss < -3.0;
            (is_correct, profit_loss, is_fatal)
        }
        "sell" => {
            let is_correct = best_price < current_price;
            let profit_loss = ((current_price - best_price) / current_price) * 100.0;
            let is_fatal = profit_loss < -3.0;
            (is_correct, profit_loss, is_fatal)
        }
        "hold" => {
            let price_change = ((best_price - current_price) / current_price).abs() * 100.0;
            let is_correct = price_change < 1.0;
            (is_correct, 0.0, false)
        }
        _ => (false, 0.0, false),
    }
}

fn generate_test_report(results: Vec<DecisionResult>, config: Config, pairs: Vec<String>, test_period_from: String, test_period_to: String) -> TestReport {
    let total_decisions = results.len();
    let buy_decisions = results.iter().filter(|r| r.decision.action == "buy").count();
    let sell_decisions = results.iter().filter(|r| r.decision.action == "sell").count();
    let hold_decisions = results.iter().filter(|r| r.decision.action == "hold").count();

    // Calculate accuracy metrics
    let correct_30m = results.iter().filter(|r| r.was_correct_30m == Some(true)).count();
    let correct_1h = results.iter().filter(|r| r.was_correct_1h == Some(true)).count();
    let correct_2h = results.iter().filter(|r| r.was_correct_2h == Some(true)).count();

    let total_with_30m = results.iter().filter(|r| r.was_correct_30m.is_some()).count();
    let total_with_1h = results.iter().filter(|r| r.was_correct_1h.is_some()).count();
    let total_with_2h = results.iter().filter(|r| r.was_correct_2h.is_some()).count();

    let accuracy_30m = if total_with_30m > 0 { (correct_30m as f64 / total_with_30m as f64) * 100.0 } else { 0.0 };
    let accuracy_1h = if total_with_1h > 0 { (correct_1h as f64 / total_with_1h as f64) * 100.0 } else { 0.0 };
    let accuracy_2h = if total_with_2h > 0 { (correct_2h as f64 / total_with_2h as f64) * 100.0 } else { 0.0 };

    // Calculate profit/loss metrics
    let total_profit_30m: f64 = results.iter().filter_map(|r| r.profit_loss_30m).sum();
    let total_profit_1h: f64 = results.iter().filter_map(|r| r.profit_loss_1h).sum();
    let total_profit_2h: f64 = results.iter().filter_map(|r| r.profit_loss_2h).sum();

    let avg_profit_30m = if total_with_30m > 0 { total_profit_30m / total_with_30m as f64 } else { 0.0 };
    let avg_profit_1h = if total_with_1h > 0 { total_profit_1h / total_with_1h as f64 } else { 0.0 };
    let avg_profit_2h = if total_with_2h > 0 { total_profit_2h / total_with_2h as f64 } else { 0.0 };

    let best_profit_30m = results.iter().filter_map(|r| r.profit_loss_30m).fold(f64::NEG_INFINITY, f64::max);
    let best_profit_1h = results.iter().filter_map(|r| r.profit_loss_1h).fold(f64::NEG_INFINITY, f64::max);
    let best_profit_2h = results.iter().filter_map(|r| r.profit_loss_2h).fold(f64::NEG_INFINITY, f64::max);

    let worst_loss_30m = results.iter().filter_map(|r| r.profit_loss_30m).fold(f64::INFINITY, f64::min);
    let worst_loss_1h = results.iter().filter_map(|r| r.profit_loss_1h).fold(f64::INFINITY, f64::min);
    let worst_loss_2h = results.iter().filter_map(|r| r.profit_loss_2h).fold(f64::INFINITY, f64::min);

    // Generate per-pair statistics
    let mut pair_stats = HashMap::new();
    for pair in &pairs {
        let pair_results: Vec<_> = results.iter().filter(|r| r.pair == *pair).collect();
        if !pair_results.is_empty() {
            let pair_decisions = pair_results.len();
            let pair_buy = pair_results.iter().filter(|r| r.decision.action == "buy").count();
            let pair_sell = pair_results.iter().filter(|r| r.decision.action == "sell").count();
            let pair_hold = pair_results.iter().filter(|r| r.decision.action == "hold").count();

            let pair_correct_30m = pair_results.iter().filter(|r| r.was_correct_30m == Some(true)).count();
            let pair_total_30m = pair_results.iter().filter(|r| r.was_correct_30m.is_some()).count();
            let pair_accuracy_30m = if pair_total_30m > 0 { (pair_correct_30m as f64 / pair_total_30m as f64) * 100.0 } else { 0.0 };

            let pair_profit_30m: f64 = pair_results.iter().filter_map(|r| r.profit_loss_30m).sum();

            pair_stats.insert(pair.clone(), PairStatistics {
                pair: pair.clone(),
                decisions: pair_decisions,
                buy_decisions: pair_buy,
                sell_decisions: pair_sell,
                hold_decisions: pair_hold,
                accuracy_30m: pair_accuracy_30m,
                accuracy_1h: 0.0, // Simplified for now
                accuracy_2h: 0.0, // Simplified for now
                profit_loss_30m: pair_profit_30m,
                profit_loss_1h: 0.0, // Simplified for now
                profit_loss_2h: 0.0, // Simplified for now
            });
        }
    }

    TestReport {
        pairs,
        model: config.openai_model.clone(),
        test_period_from,
        test_period_to,
        config,
        total_decisions,
        buy_decisions,
        sell_decisions,
        hold_decisions,
        pair_stats,
        correct_predictions_30m: correct_30m,
        correct_predictions_1h: correct_1h,
        correct_predictions_2h: correct_2h,
        wrong_predictions_30m: total_with_30m - correct_30m,
        wrong_predictions_1h: total_with_1h - correct_1h,
        wrong_predictions_2h: total_with_2h - correct_2h,
        total_profit_loss_30m: total_profit_30m,
        total_profit_loss_1h: total_profit_1h,
        total_profit_loss_2h: total_profit_2h,
        average_profit_per_decision_30m: avg_profit_30m,
        average_profit_per_decision_1h: avg_profit_1h,
        average_profit_per_decision_2h: avg_profit_2h,
        best_decision_profit_30m: best_profit_30m,
        best_decision_profit_1h: best_profit_1h,
        best_decision_profit_2h: best_profit_2h,
        worst_decision_loss_30m: worst_loss_30m,
        worst_decision_loss_1h: worst_loss_1h,
        worst_decision_loss_2h: worst_loss_2h,
        accuracy_30m,
        accuracy_1h,
        accuracy_2h,
        buy_accuracy_30m: 0.0, // Simplified for now
        buy_accuracy_1h: 0.0,
        buy_accuracy_2h: 0.0,
        sell_accuracy_30m: 0.0,
        sell_accuracy_1h: 0.0,
        sell_accuracy_2h: 0.0,
        initial_portfolio_value: 10000.0,
        final_portfolio_value_30m: 10000.0,
        final_portfolio_value_1h: 10000.0,
        final_portfolio_value_2h: 10000.0,
        portfolio_return_30m: 0.0,
        portfolio_return_1h: 0.0,
        portfolio_return_2h: 0.0,
        results,
    }
}

fn print_test_report(report: &TestReport) {
    println!("\n🎯 MULTI-PAIR TRADING TEST RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    println!("📊 Test Configuration:");
    println!("   Pairs: {}", report.pairs.join(", "));
    println!("   Model: {}", report.model);
    println!("   Period: {} to {}", report.test_period_from, report.test_period_to);
    println!("   Total Decisions: {}", report.total_decisions);
    
    println!("\n📈 Decision Breakdown:");
    println!("   🟢 Buy:  {} ({:.1}%)", report.buy_decisions, (report.buy_decisions as f64 / report.total_decisions as f64) * 100.0);
    println!("   🔴 Sell: {} ({:.1}%)", report.sell_decisions, (report.sell_decisions as f64 / report.total_decisions as f64) * 100.0);
    println!("   ⚪ Hold: {} ({:.1}%)", report.hold_decisions, (report.hold_decisions as f64 / report.total_decisions as f64) * 100.0);
    
    println!("\n🎯 Overall Accuracy:");
    println!("   30-minute: {:.1}%", report.accuracy_30m);
    println!("   1-hour:    {:.1}%", report.accuracy_1h);
    println!("   2-hour:    {:.1}%", report.accuracy_2h);
    
    println!("\n💰 Profitability:");
    println!("   30-minute: {:+.2}% (avg: {:+.2}%)", report.total_profit_loss_30m, report.average_profit_per_decision_30m);
    println!("   1-hour:    {:+.2}% (avg: {:+.2}%)", report.total_profit_loss_1h, report.average_profit_per_decision_1h);
    println!("   2-hour:    {:+.2}% (avg: {:+.2}%)", report.total_profit_loss_2h, report.average_profit_per_decision_2h);
    
    println!("\n📊 Per-Pair Performance:");
    for (pair, stats) in &report.pair_stats {
        println!("   {}: {} decisions, {:.1}% accuracy, {:+.2}% profit",
                 pair, stats.decisions, stats.accuracy_30m, stats.profit_loss_30m);
    }
    
    println!("\n⚖️  Risk/Reward:");
    println!("   Best Decision:  {:+.2}%", report.best_decision_profit_30m);
    println!("   Worst Decision: {:+.2}%", report.worst_decision_loss_30m);
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logger();
    
    dotenv::dotenv()
        .wrap_err("Failed to load .env file")
        .with_suggestion(|| "Make sure .env file exists and is readable")?;
    
    let config = (&*botmarley::config::CONFIG).clone();
    let allowed_pairs = if config.allowed_pairs.is_empty() {
        vec!["BTCUSDC".to_string()]
    } else {
        config.pairs()
    };
    
    let system_message = get_system_message();
    let test_period_from = "2024-10-01";
    let test_period_to = "2024-10-02";
    
    println!("🚀 Starting Multi-Pair Trading Decision Analysis");
    println!("Pairs: {}", allowed_pairs.join(", "));
    println!("Period: {} to {}", test_period_from, test_period_to);
    println!("Model: {}", config.openai_model);
    
    // Load klines data for all pairs
    let mut all_klines = HashMap::new();
    let mut min_length = usize::MAX;
    
    // Create single data collector
    let collector = DataCollector::new(config.clone())?;
    
    println!("\n📊 Loading klines data for all pairs...");
    for pair in &allowed_pairs {
        info!("Loading data for pair: {}", pair);
        match collector.get_klines_for_symbol(pair.clone()).await {
            Ok(klines) => {
                let filtered_klines = filter_klines_by_date_range(klines, test_period_from, test_period_to).await?;
                if !filtered_klines.is_empty() {
                    min_length = min_length.min(filtered_klines.len());
                    all_klines.insert(pair.clone(), filtered_klines);
                    println!("✅ Loaded {} klines for {}", all_klines[pair].len(), pair);
                }
            }
            Err(e) => {
                println!("❌ Failed to load klines for {}: {}", pair, e);
                continue;
            }
        }
    }
    
    if all_klines.is_empty() {
        println!("❌ No klines data loaded for any pairs");
        return Ok(());
    }
    
    println!("📈 Processing {} pairs with minimum {} klines", all_klines.len(), min_length);
    
    let mut portfolio = MultiPairPortfolio::new(10000.0);
    let mut results = Vec::new();
    
    // Process each time point (skip the last 24 to ensure we have future data for evaluation)
    let process_count = if min_length > 24 { min_length - 24 } else { 0 };
    
    for i in 20..process_count.min(50) { // Limit to 30 decisions for demo
        println!("\n🔄 Processing decision {}/{}", i - 19, process_count.min(50) - 20);
        
        // Get current prices for all pairs
        let mut current_prices = HashMap::new();
        for (pair, klines) in &all_klines {
            if i < klines.len() {
                current_prices.insert(pair.clone(), klines[i].close);
            }
        }
        
        // Generate user message with all pairs data
        let user_message = generate_multi_pair_user_message(&all_klines, i, &portfolio, &allowed_pairs);
        
        // Make LLM request
        match make_llm_request(&config, &system_message, &user_message).await {
            Ok(decision) => {
                // Validate that the decision pair exists in our data
                if !all_klines.contains_key(&decision.pair) {
                    println!("❌ LLM returned invalid pair '{}', skipping decision", decision.pair);
                    continue;
                }
                
                let current_price = *current_prices.get(&decision.pair).unwrap_or(&0.0);
                if current_price == 0.0 {
                    println!("❌ No price data for pair '{}', skipping decision", decision.pair);
                    continue;
                }
                
                println!("🤖 Decision: {} {} {:.6} @ ${:.4} (confidence: {:.0}%)",
                         decision.action.to_uppercase(),
                         decision.pair,
                         decision.amount,
                         current_price,
                         decision.confidence * 100.0);
                println!("💭 Reasoning: {}", decision.reasoning);
                
                let pair_klines = all_klines.get(&decision.pair).unwrap();
                let timestamp = pair_klines[i].open_time;
                
                // Execute the trade in portfolio
                portfolio.execute_trade(&decision, &current_prices, timestamp);
                
                // Get future prices for evaluation
                let future_30m_idx = i + 6; // 30 minutes = 6 * 5-minute candles
                let future_1h_idx = i + 12; // 1 hour = 12 * 5-minute candles
                let future_2h_idx = i + 24; // 2 hours = 24 * 5-minute candles
                
                let actual_price_30m = if future_30m_idx < pair_klines.len() {
                    Some(pair_klines[future_30m_idx].close)
                } else { None };
                
                let actual_price_1h = if future_1h_idx < pair_klines.len() {
                    Some(pair_klines[future_1h_idx].close)
                } else { None };
                
                let actual_price_2h = if future_2h_idx < pair_klines.len() {
                    Some(pair_klines[future_2h_idx].close)
                } else { None };
                
                // Evaluate decisions
                let (was_correct_30m, profit_loss_30m) = if let Some(future_price) = actual_price_30m {
                    let (correct, profit, _) = evaluate_decision(&decision, current_price, future_price);
                    (Some(correct), Some(profit))
                } else { (None, None) };
                
                let (was_correct_1h, profit_loss_1h) = if let Some(future_price) = actual_price_1h {
                    let (correct, profit, _) = evaluate_decision(&decision, current_price, future_price);
                    (Some(correct), Some(profit))
                } else { (None, None) };
                
                let (was_correct_2h, profit_loss_2h) = if let Some(future_price) = actual_price_2h {
                    let (correct, profit, _) = evaluate_decision(&decision, current_price, future_price);
                    (Some(correct), Some(profit))
                } else { (None, None) };
                
                let result = DecisionResult {
                    timestamp,
                    pair: decision.pair.clone(),
                    price: current_price,
                    decision,
                    actual_price_30m,
                    actual_price_1h,
                    actual_price_2h,
                    was_correct_30m,
                    was_correct_1h,
                    was_correct_2h,
                    profit_loss_30m,
                    profit_loss_1h,
                    profit_loss_2h,
                };
                
                results.push(result);
                
                // Small delay to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                println!("❌ Failed to get decision for index {}: {}", i, e);
            }
        }
    }
    
    // Generate and print report
    let report = generate_test_report(results, config, allowed_pairs, test_period_from.to_string(), test_period_to.to_string());
    print_test_report(&report);
    
    println!("\n✅ Multi-pair trading test completed!");
    
    Ok(())
}