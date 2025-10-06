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

#[derive(Debug, Serialize, Deserialize)]
struct TradingDecision {
    action: String, // "buy", "sell", "hold"
    confidence: f64, // 0.0 to 1.0
    reasoning: String,
    thinking:Vec<Thought>,
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
    // temperature: f64,
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
    symbol: String,
    model: String,
    test_period_from: String,
    test_period_to: String,
    config: Config,
    
    // Basic statistics
    total_decisions: usize,
    buy_decisions: usize,
    sell_decisions: usize,
    hold_decisions: usize,
    
    // Accuracy metrics
    correct_predictions_30m: usize,
    correct_predictions_1h: usize,
    correct_predictions_2h: usize,
    wrong_predictions_30m: usize,
    wrong_predictions_1h: usize,
    wrong_predictions_2h: usize,
    
    // Decision-specific performance
    good_buy_decisions_30m: usize,
    good_buy_decisions_1h: usize,
    good_buy_decisions_2h: usize,
    bad_buy_decisions_30m: usize,
    bad_buy_decisions_1h: usize,
    bad_buy_decisions_2h: usize,
    good_sell_decisions_30m: usize,
    good_sell_decisions_1h: usize,
    good_sell_decisions_2h: usize,
    bad_sell_decisions_30m: usize,
    bad_sell_decisions_1h: usize,
    bad_sell_decisions_2h: usize,
    
    // Fatal decisions (buy when price drops significantly)
    fatal_buy_decisions_30m: usize,
    fatal_buy_decisions_1h: usize,
    fatal_buy_decisions_2h: usize,
    fatal_sell_decisions_30m: usize,
    fatal_sell_decisions_1h: usize,
    fatal_sell_decisions_2h: usize,
    
    // Profitability
    profitable_decisions_30m: usize,
    profitable_decisions_1h: usize,
    profitable_decisions_2h: usize,
    losing_decisions_30m: usize,
    losing_decisions_1h: usize,
    losing_decisions_2h: usize,
    
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
struct PortfolioSimulation {
    initial_value: f64,
    current_value: f64,
    current_position: String, // "cash", "crypto"
    crypto_amount: f64,
    cash_amount: f64,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
struct Transaction {
    timestamp: i64,
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
    "required": ["thinking", "reasoning", "action", "confidence" ],  
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
        // temperature: 0.1,
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
#[instrument(ret)]
fn generate_user_message(klines: &[botmarley::binance::data_collector::KlineData], index: usize) -> String {
    if klines.is_empty() || index >= klines.len() {
        return "No kline data available".to_string();
    }

    // Get current and previous klines for context
    let current = &klines[index];
    let start_idx = if index >= 20 { index - 20 } else { 0 };
    let context_klines = &klines[start_idx..=index];

    // Calculate technical indicators
    let closes: Vec<f64> = context_klines.iter().map(|k| k.close).collect();
    let highs: Vec<f64> = context_klines.iter().map(|k| k.high).collect();
    let lows: Vec<f64> = context_klines.iter().map(|k| k.low).collect();
    let volumes: Vec<f64> = context_klines.iter().map(|k| k.volume).collect();

    let mut message = format!(
        "Current Market Data for {}:\n\n",
        current.symbol
    );

    message.push_str(&format!(
        "Current Price: ${:.2}\n",
        current.close
    ));

    message.push_str(&format!(
        "Current Candle: Open: ${:.2}, High: ${:.2}, Low: ${:.2}, Close: ${:.2}\n",
        current.open, current.high, current.low, current.close
    ));

    message.push_str(&format!(
        "Volume: {:.2}\n",
        current.volume
    ));

    message.push_str(&format!(
        "Time: {}\n\n",
        DateTime::<Utc>::from_timestamp_millis(current.open_time)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Add technical indicators if we have enough data
    if closes.len() >= 14 {
        let rsi = relative_strength_index(&closes, 14);
        if let Some(current_rsi) = rsi.last().and_then(|r| *r) {
            message.push_str(&format!("RSI (14): {:.2}\n", current_rsi));
        }

        let mfi = money_flow_index(&highs, &lows, &closes, &volumes, 14);
        if let Some(current_mfi) = mfi.last().and_then(|m| *m) {
            message.push_str(&format!("MFI (14): {:.2}\n", current_mfi));
        }
    }

    // Calculate multiple SMA and EMA periods
    if closes.len() >= 6 {
        let sma_6 = simple_moving_average(&closes, 6);
        if let Some(current_sma_6) = sma_6.last().and_then(|s| *s) {
            message.push_str(&format!("SMA (6): ${:.2}\n", current_sma_6));
        }

        let ema_6 = exponential_moving_average(&closes, 6);
        if let Some(current_ema_6) = ema_6.last().and_then(|e| *e) {
            message.push_str(&format!("EMA (6): ${:.2}\n", current_ema_6));
        }
    }

    if closes.len() >= 14 {
        let sma_14 = simple_moving_average(&closes, 14);
        if let Some(current_sma_14) = sma_14.last().and_then(|s| *s) {
            message.push_str(&format!("SMA (14): ${:.2}\n", current_sma_14));
        }

        let ema_14 = exponential_moving_average(&closes, 14);
        if let Some(current_ema_14) = ema_14.last().and_then(|e| *e) {
            message.push_str(&format!("EMA (14): ${:.2}\n", current_ema_14));
        }
    }

    if closes.len() >= 20 {
        let sma_20 = simple_moving_average(&closes, 20);
        if let Some(current_sma_20) = sma_20.last().and_then(|s| *s) {
            message.push_str(&format!("SMA (20): ${:.2}\n", current_sma_20));
        }

        let ema_20 = exponential_moving_average(&closes, 20);
        if let Some(current_ema_20) = ema_20.last().and_then(|e| *e) {
            message.push_str(&format!("EMA (20): ${:.2}\n", current_ema_20));
        }

        let bb = bollinger_bands(&closes, 20, 2.0);
        if let (Some(upper), Some(middle), Some(lower)) = (
            bb.0.last().and_then(|u| *u),
            bb.1.last().and_then(|m| *m),
            bb.2.last().and_then(|l| *l),
        ) {
            message.push_str(&format!(
                "Bollinger Bands: Upper: ${:.2}, Middle: ${:.2}, Lower: ${:.2}\n",
                upper, middle, lower
            ));
        }
    }

    // Add MACD
    if closes.len() >= 26 {
        let macd_values = MACD::new(&closes, 12, 26, 9);
        if let Some(current_macd) = macd_values.last() {
            message.push_str(&format!(
                "MACD: {:.4} SIGNAL: {:.4} HISTOGRAM: {:.4}\n",
                current_macd.macd, current_macd.signal, current_macd.histogram
            ));
        }
    }

    // Add recent price context
    message.push_str("\nRecent Price (last 5 candles):\n");
    let recent_start = if index >= 4 { index - 4 } else { 0 };
    for i in recent_start..=index {
        let k = &klines[i];
        let change = if i > 0 {
            let prev_close = klines[i - 1].close;
            ((k.close - prev_close) / prev_close) * 100.0
        } else {
            0.0
        };
        
        message.push_str(&format!(
            "  {}: ${:.2} ({:+.2}%)\n",
            DateTime::<Utc>::from_timestamp_millis(k.open_time)
                .unwrap_or_default()
                .format("%H:%M"),
            k.close,
            change
        ));
    }

    // Add recent MACD context
    if closes.len() >= 26 {
        message.push_str("\nRecent MACD (last 5 candles):\n");
        let macd_values = MACD::new(&closes, 12, 26, 9);
        
        // Calculate the correct indices for recent MACD values
        let _context_len = context_klines.len();
        let macd_len = macd_values.len();
        
        for i in recent_start..=index {
            let context_idx = i - start_idx;
            if context_idx < macd_len {
                let k = &klines[i];
                let macd = &macd_values[context_idx];
                message.push_str(&format!(
                    "  {}: {:.4} ( signal: {:.4} histogram: {:.4} )\n",
                    DateTime::<Utc>::from_timestamp_millis(k.open_time)
                        .unwrap_or_default()
                        .format("%H:%M"),
                    macd.macd,
                    macd.signal,
                    macd.histogram
                ));
            }
        }
    }

    // Add recent MA context
    message.push_str("\nRecent MA (last 5 candles):\n");
    let sma_6 = if closes.len() >= 6 { simple_moving_average(&closes, 6) } else { vec![] };
    let sma_14 = if closes.len() >= 14 { simple_moving_average(&closes, 14) } else { vec![] };
    let sma_20 = if closes.len() >= 20 { simple_moving_average(&closes, 20) } else { vec![] };
    let ema_6 = if closes.len() >= 6 { exponential_moving_average(&closes, 6) } else { vec![] };
    let ema_14 = if closes.len() >= 14 { exponential_moving_average(&closes, 14) } else { vec![] };
    let ema_20 = if closes.len() >= 20 { exponential_moving_average(&closes, 20) } else { vec![] };

    for i in recent_start..=index {
        let k = &klines[i];
        let context_idx = i - start_idx;
        
        let sma_6_val = if context_idx < sma_6.len() {
            sma_6[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };
        
        let sma_14_val = if context_idx < sma_14.len() {
            sma_14[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };
        
        let sma_20_val = if context_idx < sma_20.len() {
            sma_20[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };
        
        let ema_6_val = if context_idx < ema_6.len() {
            ema_6[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };
        
        let ema_14_val = if context_idx < ema_14.len() {
            ema_14[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };
        
        let ema_20_val = if context_idx < ema_20.len() {
            ema_20[context_idx].map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".to_string())
        } else { "N/A".to_string() };

        message.push_str(&format!(
            "  {}: SMA {{ 6: {}, 14: {}, 20: {} }}, EMA {{ 6: {}, 14: {}, 20: {} }}\n",
            DateTime::<Utc>::from_timestamp_millis(k.open_time)
                .unwrap_or_default()
                .format("%H:%M"),
            sma_6_val, sma_14_val, sma_20_val,
            ema_6_val, ema_14_val, ema_20_val
        ));
    }

    message.push_str("\nPlease analyze this data and provide a trading decision (buy, sell, or hold) with your confidence level and reasoning. Consider the technical indicators, recent price, and overall market context.");
    dbg!(&message);
    message
}

async fn filter_klines_by_date_range(
    klines: Vec<botmarley::binance::data_collector::KlineData>,
    start_date: &str,
    end_date: &str,
) -> color_eyre::Result<Vec<botmarley::binance::data_collector::KlineData>> {
    let start_timestamp = date_string_to_timestamp(start_date)? * 1000; // Convert to milliseconds
    let end_timestamp = date_string_to_timestamp(end_date)? * 1000; // Convert to milliseconds

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
) -> (bool, f64, bool) { // Added fatal flag
    match decision.action.as_str() {
        "buy" => {
            // For buy decisions, best_price should be the maximum high price reached in the period
            let is_correct = best_price > current_price;
            let profit_loss = ((best_price - current_price) / current_price) * 100.0;
            let is_fatal = profit_loss < -3.0; // Fatal if loss > 3%
            (is_correct, profit_loss, is_fatal)
        }
        "sell" => {
            // For sell decisions, best_price should be the minimum low price reached in the period
            let is_correct = best_price < current_price;
            let profit_loss = ((current_price - best_price) / current_price) * 100.0;
            let is_fatal = profit_loss < -3.0; // Fatal if loss > 3%
            (is_correct, profit_loss, is_fatal)
        }
        "hold" => {
            // For hold, we consider it correct if price doesn't move significantly (within 1%)
            // For hold, best_price represents the final price in the period
            let price_change = ((best_price - current_price) / current_price).abs() * 100.0;
            let is_correct = price_change < 1.0;
            (is_correct, 0.0, false) // No profit/loss for hold, never fatal
        }
        _ => (false, 0.0, false),
    }
}

fn simulate_portfolio(results: &[DecisionResult], initial_value: f64) -> (PortfolioSimulation, PortfolioSimulation, PortfolioSimulation) {
    let mut sim_30m = PortfolioSimulation {
        initial_value,
        current_value: initial_value,
        current_position: "cash".to_string(),
        crypto_amount: 0.0,
        cash_amount: initial_value,
        transactions: Vec::new(),
    };
    
    let mut sim_1h = sim_30m.clone();
    let mut sim_2h = sim_30m.clone();
    
    for result in results {
        // 30-minute simulation
        if let Some(future_price_30m) = result.actual_price_30m {
            simulate_decision(&mut sim_30m, result, future_price_30m, result.timestamp);
        }
        
        // 1-hour simulation
        if let Some(future_price_1h) = result.actual_price_1h {
            simulate_decision(&mut sim_1h, result, future_price_1h, result.timestamp);
        }
        
        // 2-hour simulation
        if let Some(future_price_2h) = result.actual_price_2h {
            simulate_decision(&mut sim_2h, result, future_price_2h, result.timestamp);
        }
    }
    
    (sim_30m, sim_1h, sim_2h)
}

fn simulate_decision(portfolio: &mut PortfolioSimulation, result: &DecisionResult, future_price: f64, timestamp: i64) {
    match result.decision.action.as_str() {
        "buy" => {
            if portfolio.current_position == "cash" && portfolio.cash_amount > 0.0 {
                // Buy crypto with all cash
                let crypto_bought = portfolio.cash_amount / result.price;
                portfolio.crypto_amount = crypto_bought;
                portfolio.cash_amount = 0.0;
                portfolio.current_position = "crypto".to_string();
                
                // Calculate portfolio value at future price
                portfolio.current_value = portfolio.crypto_amount * future_price;
                
                portfolio.transactions.push(Transaction {
                    timestamp,
                    action: "buy".to_string(),
                    price: result.price,
                    amount: crypto_bought,
                    value: portfolio.cash_amount,
                    portfolio_value_after: portfolio.current_value,
                });
            }
        }
        "sell" => {
            if portfolio.current_position == "crypto" && portfolio.crypto_amount > 0.0 {
                // Sell all crypto for cash
                portfolio.cash_amount = portfolio.crypto_amount * result.price;
                portfolio.crypto_amount = 0.0;
                portfolio.current_position = "cash".to_string();
                portfolio.current_value = portfolio.cash_amount;
                
                portfolio.transactions.push(Transaction {
                    timestamp,
                    action: "sell".to_string(),
                    price: result.price,
                    amount: portfolio.crypto_amount,
                    value: portfolio.cash_amount,
                    portfolio_value_after: portfolio.current_value,
                });
            }
        }
        "hold" => {
            // Update portfolio value based on current position
            if portfolio.current_position == "crypto" {
                portfolio.current_value = portfolio.crypto_amount * future_price;
            } else {
                portfolio.current_value = portfolio.cash_amount;
            }
        }
        _ => {}
    }
}

impl Clone for PortfolioSimulation {
    fn clone(&self) -> Self {
        PortfolioSimulation {
            initial_value: self.initial_value,
            current_value: self.current_value,
            current_position: self.current_position.clone(),
            crypto_amount: self.crypto_amount,
            cash_amount: self.cash_amount,
            transactions: self.transactions.clone(),
        }
    }
}

fn print_report(report: &TestReport) {
    println!("\n{}", "=".repeat(80));
    println!("                           TRADING DECISION ANALYSIS REPORT");
    println!("{}", "=".repeat(80));
    
    println!("\n📊 SUMMARY STATISTICS:");
    println!("├─ Total Decisions: {}", report.total_decisions);
    println!("├─ Buy Decisions: {} ({:.1}%)", report.buy_decisions,
             (report.buy_decisions as f64 / report.total_decisions as f64) * 100.0);
    println!("├─ Sell Decisions: {} ({:.1}%)", report.sell_decisions,
             (report.sell_decisions as f64 / report.total_decisions as f64) * 100.0);
    println!("└─ Hold Decisions: {} ({:.1}%)", report.hold_decisions,
             (report.hold_decisions as f64 / report.total_decisions as f64) * 100.0);

    println!("\n🎯 OVERALL ACCURACY METRICS:");
    println!("├─ 30-minute: {:.1}% (✅{} / ❌{})", report.accuracy_30m,
             report.correct_predictions_30m, report.wrong_predictions_30m);
    println!("├─ 1-hour: {:.1}% (✅{} / ❌{})", report.accuracy_1h,
             report.correct_predictions_1h, report.wrong_predictions_1h);
    println!("└─ 2-hour: {:.1}% (✅{} / ❌{})", report.accuracy_2h,
             report.correct_predictions_2h, report.wrong_predictions_2h);

    println!("\n📈 BUY DECISION PERFORMANCE:");
    println!("├─ 30-minute: {:.1}% (✅{} / ❌{})", report.buy_accuracy_30m,
             report.good_buy_decisions_30m, report.bad_buy_decisions_30m);
    println!("├─ 1-hour: {:.1}% (✅{} / ❌{})", report.buy_accuracy_1h,
             report.good_buy_decisions_1h, report.bad_buy_decisions_1h);
    println!("└─ 2-hour: {:.1}% (✅{} / ❌{})", report.buy_accuracy_2h,
             report.good_buy_decisions_2h, report.bad_buy_decisions_2h);

    println!("\n📉 SELL DECISION PERFORMANCE:");
    println!("├─ 30-minute: {:.1}% (✅{} / ❌{})", report.sell_accuracy_30m,
             report.good_sell_decisions_30m, report.bad_sell_decisions_30m);
    println!("├─ 1-hour: {:.1}% (✅{} / ❌{})", report.sell_accuracy_1h,
             report.good_sell_decisions_1h, report.bad_sell_decisions_1h);
    println!("└─ 2-hour: {:.1}% (✅{} / ❌{})", report.sell_accuracy_2h,
             report.good_sell_decisions_2h, report.bad_sell_decisions_2h);

    println!("\n💰 PROFIT/LOSS ANALYSIS:");
    println!("├─ 30-minute: Total: {:+.2}% | Avg: {:+.2}% | Best: {:+.2}% | Worst: {:+.2}%",
             report.total_profit_loss_30m, report.average_profit_per_decision_30m,
             report.best_decision_profit_30m, report.worst_decision_loss_30m);
    println!("├─ 1-hour: Total: {:+.2}% | Avg: {:+.2}% | Best: {:+.2}% | Worst: {:+.2}%",
             report.total_profit_loss_1h, report.average_profit_per_decision_1h,
             report.best_decision_profit_1h, report.worst_decision_loss_1h);
    println!("└─ 2-hour: Total: {:+.2}% | Avg: {:+.2}% | Best: {:+.2}% | Worst: {:+.2}%",
             report.total_profit_loss_2h, report.average_profit_per_decision_2h,
             report.best_decision_profit_2h, report.worst_decision_loss_2h);

    println!("\n💸 PROFITABILITY BREAKDOWN:");
    println!("├─ 30-minute: 💚{} profitable | 💔{} losing",
             report.profitable_decisions_30m, report.losing_decisions_30m);
    println!("├─ 1-hour: 💚{} profitable | 💔{} losing",
             report.profitable_decisions_1h, report.losing_decisions_1h);
    println!("└─ 2-hour: 💚{} profitable | 💔{} losing",
             report.profitable_decisions_2h, report.losing_decisions_2h);

    println!("\n⚠️ FATAL DECISIONS (>3% Loss):");
    println!("├─ Fatal Buy Decisions: 30m: {} | 1h: {} | 2h: {}",
             report.fatal_buy_decisions_30m, report.fatal_buy_decisions_1h, report.fatal_buy_decisions_2h);
    println!("└─ Fatal Sell Decisions: 30m: {} | 1h: {} | 2h: {}",
             report.fatal_sell_decisions_30m, report.fatal_sell_decisions_1h, report.fatal_sell_decisions_2h);

    println!("\n💰 PORTFOLIO SIMULATION (${:.2} initial):", report.initial_portfolio_value);
    println!("├─ 30-minute: ${:.2} ({:+.2}%)",
             report.final_portfolio_value_30m, report.portfolio_return_30m);
    println!("├─ 1-hour: ${:.2} ({:+.2}%)",
             report.final_portfolio_value_1h, report.portfolio_return_1h);
    println!("└─ 2-hour: ${:.2} ({:+.2}%)",
             report.final_portfolio_value_2h, report.portfolio_return_2h);

    println!("\n📈 DETAILED RESULTS TABLE:");
    println!("{:<20} {:<8} {:<8} {:<6} {:<10} {:<10} {:<10} {:<8} {:<8} {:<8}",
             "Timestamp", "Price", "Action", "Conf", "30m P&L", "1h P&L", "2h P&L", "30m ✓", "1h ✓", "2h ✓");
    println!("{}", "-".repeat(100));

    for result in &report.results {
        let timestamp = DateTime::<Utc>::from_timestamp_millis(result.timestamp)
            .unwrap_or_default()
            .format("%m-%d %H:%M");
        
        println!("{:<20} ${:<7.2} {:<8} {:<5.0}% {:>+9.2}% {:>+9.2}% {:>+9.2}% {:<8} {:<8} {:<8}",
                 timestamp,
                 result.price,
                 result.decision.action.to_uppercase(),
                 result.decision.confidence * 100.0,
                 result.profit_loss_30m.unwrap_or(0.0),
                 result.profit_loss_1h.unwrap_or(0.0),
                 result.profit_loss_2h.unwrap_or(0.0),
                 if result.was_correct_30m.unwrap_or(false) { "✅" } else { "❌" },
                 if result.was_correct_1h.unwrap_or(false) { "✅" } else { "❌" },
                 if result.was_correct_2h.unwrap_or(false) { "✅" } else { "❌" },
        );
    }

    println!("\n{}", "=".repeat(80));
    
    // Enhanced performance insights
    println!("\n🔍 PERFORMANCE INSIGHTS:");
    
    // Overall performance
    if report.accuracy_30m > 60.0 {
        println!("✅ Strong short-term prediction accuracy (30m: {:.1}%)", report.accuracy_30m);
    } else {
        println!("⚠️  Room for improvement in short-term predictions (30m: {:.1}%)", report.accuracy_30m);
    }
    
    // Profitability insights
    if report.total_profit_loss_2h > 0.0 {
        println!("✅ Positive overall returns over 2-hour periods ({:+.2}%)", report.total_profit_loss_2h);
    } else {
        println!("⚠️  Negative overall returns - strategy needs refinement ({:+.2}%)", report.total_profit_loss_2h);
    }
    
    // Best timeframe
    let best_timeframe = if report.accuracy_2h >= report.accuracy_1h && report.accuracy_2h >= report.accuracy_30m {
        "2-hour"
    } else if report.accuracy_1h >= report.accuracy_30m {
        "1-hour"
    } else {
        "30-minute"
    };
    println!("📊 Best performing timeframe: {}", best_timeframe);
    
    // Action-specific insights
    if report.buy_decisions > 0 {
        let best_buy_timeframe = if report.buy_accuracy_2h >= report.buy_accuracy_1h && report.buy_accuracy_2h >= report.buy_accuracy_30m {
            ("2-hour", report.buy_accuracy_2h)
        } else if report.buy_accuracy_1h >= report.buy_accuracy_30m {
            ("1-hour", report.buy_accuracy_1h)
        } else {
            ("30-minute", report.buy_accuracy_30m)
        };
        println!("🟢 Buy decisions perform best at {} timeframe ({:.1}% accuracy)", best_buy_timeframe.0, best_buy_timeframe.1);
    }
    
    if report.sell_decisions > 0 {
        let best_sell_timeframe = if report.sell_accuracy_2h >= report.sell_accuracy_1h && report.sell_accuracy_2h >= report.sell_accuracy_30m {
            ("2-hour", report.sell_accuracy_2h)
        } else if report.sell_accuracy_1h >= report.sell_accuracy_30m {
            ("1-hour", report.sell_accuracy_1h)
        } else {
            ("30-minute", report.sell_accuracy_30m)
        };
        println!("🔴 Sell decisions perform best at {} timeframe ({:.1}% accuracy)", best_sell_timeframe.0, best_sell_timeframe.1);
    }
    
    // Risk assessment
    let max_loss = report.worst_decision_loss_30m.min(report.worst_decision_loss_1h).min(report.worst_decision_loss_2h);
    let max_gain = report.best_decision_profit_30m.max(report.best_decision_profit_1h).max(report.best_decision_profit_2h);
    println!("⚖️  Risk/Reward: Max gain {:+.2}% | Max loss {:+.2}%", max_gain, max_loss);
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;
    
    // Initialize logger first
    init_logger();
    
    dotenv::dotenv()
        .wrap_err("Failed to load .env file")
        .with_suggestion(|| "Make sure .env file exists and is readable")?;
    
    let config = (&*botmarley::config::CONFIG).clone();

    // Create data collector
    let collector = DataCollector::new(config.clone())?;

    let system_message = get_system_message();
    let symbol = "BTCUSDC";
    // let test_period_from = "2024-10-04";
    let test_period_from = "2024-10-01";

    let test_period_to = "2024-10-02";
    
    println!("🚀 Starting BTC Trading Decision Analysis");
    println!("Symbol: {}", symbol);
    println!("Period: {} to {}", test_period_from, test_period_to);
    println!("System Message: {}", system_message);
    
    // Get all klines for the symbol
    println!("\n📊 Loading klines data...");
    let all_klines = collector.get_klines_for_symbol(symbol.to_string()).await?;
    println!("Loaded {} total klines", all_klines.len());
    
    // Filter klines by date range
    let klines = filter_klines_by_date_range(all_klines, test_period_from, test_period_to).await?;
    
    if klines.is_empty() {
        println!("❌ No klines found for the specified date range");
        return Ok(());
    }
    
    println!("📈 Processing {} klines in date range", klines.len());
    
    let mut results = Vec::new();
    let mut _total_requests = 0;
    
    // Process each kline (skip the last 24 to ensure we have future data for evaluation)
    let process_count = if klines.len() > 24 { klines.len() - 24 } else { 0 };
    
    for i in 20..process_count { // Start from 20 to have enough historical data for indicators
        _total_requests += 1;
        println!("\n🔄 Processing kline {}/{} ({})", 
                 i + 1, process_count, 
                 DateTime::<Utc>::from_timestamp_millis(klines[i].open_time)
                     .unwrap_or_default()
                     .format("%Y-%m-%d %H:%M:%S UTC"));
        
        let user_message = generate_user_message(&klines, i);
        
        // Make LLM request
        match make_llm_request(&config, &system_message, &user_message).await {
            Ok(decision) => {
                println!("🤖 Decision: {} (confidence: {:.0}%)", 
                         decision.action.to_uppercase(), decision.confidence * 100.0);
                println!("💭 Reasoning: {}", decision.reasoning);
                println!("{}",serde_json::to_string_pretty(&decision).unwrap());
                println!("----------------------");
                let current_price = klines[i].close;
                
                // Get future prices for evaluation
                let future_30m_idx = i + 6; // 30 minutes = 6 * 5-minute candles
                let future_1h_idx = i + 12; // 1 hour = 12 * 5-minute candles
                let future_2h_idx = i + 24; // 2 hours = 24 * 5-minute candles
                
                let mut result = DecisionResult {
                    timestamp: klines[i].open_time,
                    price: current_price,
                    decision,
                    actual_price_30m: None,
                    actual_price_1h: None,
                    actual_price_2h: None,
                    was_correct_30m: None,
                    was_correct_1h: None,
                    was_correct_2h: None,
                    profit_loss_30m: None,
                    profit_loss_1h: None,
                    profit_loss_2h: None,
                };
                
                // Evaluate 30-minute prediction
                if future_30m_idx < klines.len() {
                    let best_price = match result.decision.action.as_str() {
                        "buy" => {
                            // For buy decisions, find the maximum high price in the 30m window
                            klines.iter()
                                .skip(i + 1)
                                .take(6) // 30 minutes = 6 * 5-minute candles
                                .map(|k| k.high)
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                        "sell" => {
                            // For sell decisions, find the minimum low price in the 30m window
                            klines.iter()
                                .skip(i + 1)
                                .take(6)
                                .map(|k| k.low)
                                .fold(f64::INFINITY, f64::min)
                        }
                        _ => klines[future_30m_idx].close, // For hold, use closing price
                    };
                    result.actual_price_30m = Some(best_price);
                    let (correct, pl, _fatal) = evaluate_decision(&result.decision, current_price, best_price);
                    result.was_correct_30m = Some(correct);
                    result.profit_loss_30m = Some(pl);
                }
                
                // Evaluate 1-hour prediction
                if future_1h_idx < klines.len() {
                    let best_price = match result.decision.action.as_str() {
                        "buy" => {
                            // For buy decisions, find the maximum high price in the 1h window
                            klines.iter()
                                .skip(i + 1)
                                .take(12) // 1 hour = 12 * 5-minute candles
                                .map(|k| k.high)
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                        "sell" => {
                            // For sell decisions, find the minimum low price in the 1h window
                            klines.iter()
                                .skip(i + 1)
                                .take(12)
                                .map(|k| k.low)
                                .fold(f64::INFINITY, f64::min)
                        }
                        _ => klines[future_1h_idx].close, // For hold, use closing price
                    };
                    result.actual_price_1h = Some(best_price);
                    let (correct, pl, _fatal) = evaluate_decision(&result.decision, current_price, best_price);
                    result.was_correct_1h = Some(correct);
                    result.profit_loss_1h = Some(pl);
                }
                
                // Evaluate 2-hour prediction
                if future_2h_idx < klines.len() {
                    let best_price = match result.decision.action.as_str() {
                        "buy" => {
                            // For buy decisions, find the maximum high price in the 2h window
                            klines.iter()
                                .skip(i + 1)
                                .take(24) // 2 hours = 24 * 5-minute candles
                                .map(|k| k.high)
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                        "sell" => {
                            // For sell decisions, find the minimum low price in the 2h window
                            klines.iter()
                                .skip(i + 1)
                                .take(24)
                                .map(|k| k.low)
                                .fold(f64::INFINITY, f64::min)
                        }
                        _ => klines[future_2h_idx].close, // For hold, use closing price
                    };
                    result.actual_price_2h = Some(best_price);
                    let (correct, pl, _fatal) = evaluate_decision(&result.decision, current_price, best_price);
                    result.was_correct_2h = Some(correct);
                    result.profit_loss_2h = Some(pl);
                }
                
                results.push(result);
            }
            Err(e) => {
                println!("❌ Failed to get LLM decision: {}", e);
                continue;
            }
        }
        
        // Add a small delay to avoid overwhelming the API
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Run portfolio simulation
    let initial_portfolio = 1000.0;
    let (sim_30m, sim_1h, sim_2h) = simulate_portfolio(&results, initial_portfolio);
    
    // Generate comprehensive report statistics
    let mut buy_decisions = 0;
    let mut sell_decisions = 0;
    let mut hold_decisions = 0;
    let mut correct_30m = 0;
    let mut correct_1h = 0;
    let mut correct_2h = 0;
    let mut wrong_30m = 0;
    let mut wrong_1h = 0;
    let mut wrong_2h = 0;
    let mut good_buy_30m = 0;
    let mut good_buy_1h = 0;
    let mut good_buy_2h = 0;
    let mut bad_buy_30m = 0;
    let mut bad_buy_1h = 0;
    let mut bad_buy_2h = 0;
    let mut good_sell_30m = 0;
    let mut good_sell_1h = 0;
    let mut good_sell_2h = 0;
    let mut bad_sell_30m = 0;
    let mut bad_sell_1h = 0;
    let mut bad_sell_2h = 0;
    let mut fatal_buy_30m = 0;
    let mut fatal_buy_1h = 0;
    let mut fatal_buy_2h = 0;
    let mut fatal_sell_30m = 0;
    let mut fatal_sell_1h = 0;
    let mut fatal_sell_2h = 0;
    let mut profitable_30m = 0;
    let mut profitable_1h = 0;
    let mut profitable_2h = 0;
    let mut losing_30m = 0;
    let mut losing_1h = 0;
    let mut losing_2h = 0;
    let mut total_pl_30m = 0.0;
    let mut total_pl_1h = 0.0;
    let mut total_pl_2h = 0.0;
    let mut best_profit_30m = f64::NEG_INFINITY;
    let mut best_profit_1h = f64::NEG_INFINITY;
    let mut best_profit_2h = f64::NEG_INFINITY;
    let mut worst_loss_30m = f64::INFINITY;
    let mut worst_loss_1h = f64::INFINITY;
    let mut worst_loss_2h = f64::INFINITY;
    
    for result in &results {
        match result.decision.action.as_str() {
            "buy" => {
                buy_decisions += 1;
                if result.was_correct_30m.unwrap_or(false) { good_buy_30m += 1; } else { bad_buy_30m += 1; }
                if result.was_correct_1h.unwrap_or(false) { good_buy_1h += 1; } else { bad_buy_1h += 1; }
                if result.was_correct_2h.unwrap_or(false) { good_buy_2h += 1; } else { bad_buy_2h += 1; }
                
                // Check for fatal buy decisions
                if let Some(pl) = result.profit_loss_30m {
                    if pl < -3.0 { fatal_buy_30m += 1; }
                }
                if let Some(pl) = result.profit_loss_1h {
                    if pl < -3.0 { fatal_buy_1h += 1; }
                }
                if let Some(pl) = result.profit_loss_2h {
                    if pl < -3.0 { fatal_buy_2h += 1; }
                }
            },
            "sell" => {
                sell_decisions += 1;
                if result.was_correct_30m.unwrap_or(false) { good_sell_30m += 1; } else { bad_sell_30m += 1; }
                if result.was_correct_1h.unwrap_or(false) { good_sell_1h += 1; } else { bad_sell_1h += 1; }
                if result.was_correct_2h.unwrap_or(false) { good_sell_2h += 1; } else { bad_sell_2h += 1; }
                
                // Check for fatal sell decisions
                if let Some(pl) = result.profit_loss_30m {
                    if pl < -3.0 { fatal_sell_30m += 1; }
                }
                if let Some(pl) = result.profit_loss_1h {
                    if pl < -3.0 { fatal_sell_1h += 1; }
                }
                if let Some(pl) = result.profit_loss_2h {
                    if pl < -3.0 { fatal_sell_2h += 1; }
                }
            },
            "hold" => hold_decisions += 1,
            _ => {}
        }
        
        // Overall accuracy
        if result.was_correct_30m.unwrap_or(false) { correct_30m += 1; } else { wrong_30m += 1; }
        if result.was_correct_1h.unwrap_or(false) { correct_1h += 1; } else { wrong_1h += 1; }
        if result.was_correct_2h.unwrap_or(false) { correct_2h += 1; } else { wrong_2h += 1; }
        
        // Profit/Loss tracking
        if let Some(pl) = result.profit_loss_30m {
            total_pl_30m += pl;
            if pl > 0.0 { profitable_30m += 1; } else if pl < 0.0 { losing_30m += 1; }
            if pl > best_profit_30m { best_profit_30m = pl; }
            if pl < worst_loss_30m { worst_loss_30m = pl; }
        }
        if let Some(pl) = result.profit_loss_1h {
            total_pl_1h += pl;
            if pl > 0.0 { profitable_1h += 1; } else if pl < 0.0 { losing_1h += 1; }
            if pl > best_profit_1h { best_profit_1h = pl; }
            if pl < worst_loss_1h { worst_loss_1h = pl; }
        }
        if let Some(pl) = result.profit_loss_2h {
            total_pl_2h += pl;
            if pl > 0.0 { profitable_2h += 1; } else if pl < 0.0 { losing_2h += 1; }
            if pl > best_profit_2h { best_profit_2h = pl; }
            if pl < worst_loss_2h { worst_loss_2h = pl; }
        }
    }
    
    let total_decisions = results.len();
    let report = TestReport {
        symbol: symbol.to_string(),
        model: config.openai_model.clone(),
        test_period_from: test_period_from.to_string(),
        test_period_to: test_period_to.to_string(),
        config: config.clone(),
        total_decisions,
        buy_decisions,
        sell_decisions,
        hold_decisions,
        correct_predictions_30m: correct_30m,
        correct_predictions_1h: correct_1h,
        correct_predictions_2h: correct_2h,
        wrong_predictions_30m: wrong_30m,
        wrong_predictions_1h: wrong_1h,
        wrong_predictions_2h: wrong_2h,
        good_buy_decisions_30m: good_buy_30m,
        good_buy_decisions_1h: good_buy_1h,
        good_buy_decisions_2h: good_buy_2h,
        bad_buy_decisions_30m: bad_buy_30m,
        bad_buy_decisions_1h: bad_buy_1h,
        bad_buy_decisions_2h: bad_buy_2h,
        good_sell_decisions_30m: good_sell_30m,
        good_sell_decisions_1h: good_sell_1h,
        good_sell_decisions_2h: good_sell_2h,
        bad_sell_decisions_30m: bad_sell_30m,
        bad_sell_decisions_1h: bad_sell_1h,
        bad_sell_decisions_2h: bad_sell_2h,
        fatal_buy_decisions_30m: fatal_buy_30m,
        fatal_buy_decisions_1h: fatal_buy_1h,
        fatal_buy_decisions_2h: fatal_buy_2h,
        fatal_sell_decisions_30m: fatal_sell_30m,
        fatal_sell_decisions_1h: fatal_sell_1h,
        fatal_sell_decisions_2h: fatal_sell_2h,
        profitable_decisions_30m: profitable_30m,
        profitable_decisions_1h: profitable_1h,
        profitable_decisions_2h: profitable_2h,
        losing_decisions_30m: losing_30m,
        losing_decisions_1h: losing_1h,
        losing_decisions_2h: losing_2h,
        total_profit_loss_30m: total_pl_30m,
        total_profit_loss_1h: total_pl_1h,
        total_profit_loss_2h: total_pl_2h,
        average_profit_per_decision_30m: if total_decisions > 0 { total_pl_30m / total_decisions as f64 } else { 0.0 },
        average_profit_per_decision_1h: if total_decisions > 0 { total_pl_1h / total_decisions as f64 } else { 0.0 },
        average_profit_per_decision_2h: if total_decisions > 0 { total_pl_2h / total_decisions as f64 } else { 0.0 },
        best_decision_profit_30m: if best_profit_30m == f64::NEG_INFINITY { 0.0 } else { best_profit_30m },
        best_decision_profit_1h: if best_profit_1h == f64::NEG_INFINITY { 0.0 } else { best_profit_1h },
        best_decision_profit_2h: if best_profit_2h == f64::NEG_INFINITY { 0.0 } else { best_profit_2h },
        worst_decision_loss_30m: if worst_loss_30m == f64::INFINITY { 0.0 } else { worst_loss_30m },
        worst_decision_loss_1h: if worst_loss_1h == f64::INFINITY { 0.0 } else { worst_loss_1h },
        worst_decision_loss_2h: if worst_loss_2h == f64::INFINITY { 0.0 } else { worst_loss_2h },
        accuracy_30m: if total_decisions > 0 { (correct_30m as f64 / total_decisions as f64) * 100.0 } else { 0.0 },
        accuracy_1h: if total_decisions > 0 { (correct_1h as f64 / total_decisions as f64) * 100.0 } else { 0.0 },
        accuracy_2h: if total_decisions > 0 { (correct_2h as f64 / total_decisions as f64) * 100.0 } else { 0.0 },
        buy_accuracy_30m: if buy_decisions > 0 { (good_buy_30m as f64 / buy_decisions as f64) * 100.0 } else { 0.0 },
        buy_accuracy_1h: if buy_decisions > 0 { (good_buy_1h as f64 / buy_decisions as f64) * 100.0 } else { 0.0 },
        buy_accuracy_2h: if buy_decisions > 0 { (good_buy_2h as f64 / buy_decisions as f64) * 100.0 } else { 0.0 },
        sell_accuracy_30m: if sell_decisions > 0 { (good_sell_30m as f64 / sell_decisions as f64) * 100.0 } else { 0.0 },
        sell_accuracy_1h: if sell_decisions > 0 { (good_sell_1h as f64 / sell_decisions as f64) * 100.0 } else { 0.0 },
        sell_accuracy_2h: if sell_decisions > 0 { (good_sell_2h as f64 / sell_decisions as f64) * 100.0 } else { 0.0 },
        initial_portfolio_value: initial_portfolio,
        final_portfolio_value_30m: sim_30m.current_value,
        final_portfolio_value_1h: sim_1h.current_value,
        final_portfolio_value_2h: sim_2h.current_value,
        portfolio_return_30m: ((sim_30m.current_value - initial_portfolio) / initial_portfolio) * 100.0,
        portfolio_return_1h: ((sim_1h.current_value - initial_portfolio) / initial_portfolio) * 100.0,
        portfolio_return_2h: ((sim_2h.current_value - initial_portfolio) / initial_portfolio) * 100.0,
        results,
    };
    
    print_report(&report);
    
    // Generate HTML report
    if let Err(e) = generate_html_report(&report, &sim_30m, &sim_1h, &sim_2h) {
        println!("⚠️  Failed to generate HTML report: {}", e);
    }
    
    println!("\n✅ BotMarley trading decision analysis completed successfully!");
    Ok(())
}

fn generate_html_report(report: &TestReport, sim_30m: &PortfolioSimulation, sim_1h: &PortfolioSimulation, sim_2h: &PortfolioSimulation) -> color_eyre::Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("reports/btc_analysis_{}.html", timestamp);
    
    // Create reports directory if it doesn't exist
    std::fs::create_dir_all("reports")?;
    
    let html_content = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BTC Trading Analysis Report - {}</title>
    <style>
    .buy{{
        border-left: 6px solid green;
}}
      .sell{{
        border-left: 6px solid red;
}}
        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 0 20px rgba(0,0,0,0.1); }}
        h1 {{ color: #2c3e50; text-align: center; border-bottom: 3px solid #3498db; padding-bottom: 10px; }}
        h2 {{ color: #34495e; border-left: 4px solid #3498db; padding-left: 15px; margin-top: 30px; }}
        h3 {{ color: #7f8c8d; }}
        .config-section {{ background: #ecf0f1; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .metrics-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 20px 0; }}
        .metric-card {{ background: #fff; border: 1px solid #ddd; border-radius: 8px; padding: 15px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .metric-value {{ font-size: 24px; font-weight: bold; color: #2980b9; }}
        .metric-label {{ color: #7f8c8d; font-size: 14px; }}
        .positive {{ color: #27ae60; }}
        .negative {{ color: #e74c3c; }}
        .fatal {{ color: #c0392b; font-weight: bold; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #3498db; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        .portfolio-section {{ background: #e8f5e8; padding: 20px; border-radius: 8px; margin: 20px 0; }}
        .fatal-section {{ background: #ffeaea; padding: 20px; border-radius: 8px; margin: 20px 0; border-left: 5px solid #e74c3c; }}
        .summary-stats {{ display: flex; justify-content: space-around; flex-wrap: wrap; margin: 20px 0; }}
        .stat-item {{ text-align: center; margin: 10px; }}
        .chart-placeholder {{ background: #f8f9fa; border: 2px dashed #dee2e6; height: 300px; display: flex; align-items: center; justify-content: center; color: #6c757d; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 BTC Trading Decision Analysis Report</h1>
        <p style="text-align: center; color: #7f8c8d;">Generated on {}</p>
        
        <div class="config-section">
            <h2>📋 Test Configuration</h2>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px;">
                <div><strong>Symbol:</strong> {}</div>
                <div><strong>Model:</strong> {}</div>
                <div><strong>Period:</strong> {} to {}</div>
                <div><strong>OpenAI Base URL:</strong> {}</div>
                <div><strong>Total Decisions:</strong> {}</div>
                <div><strong>Initial Portfolio:</strong> ${:.2}</div>
            </div>
        </div>

        <h2>💰 Portfolio Simulation Results</h2>
        <div class="portfolio-section">
            <div class="summary-stats">
                <div class="stat-item">
                    <div class="metric-value">30-Minute</div>
                    <div class="metric-label">Final Value: <span class="{}">${:.2}</span></div>
                    <div class="metric-label">Return: <span class="{}">{:+.2}%</span></div>
                    <div class="metric-label">Transactions: {}</div>
                </div>
                <div class="stat-item">
                    <div class="metric-value">1-Hour</div>
                    <div class="metric-label">Final Value: <span class="{}">${:.2}</span></div>
                    <div class="metric-label">Return: <span class="{}">{:+.2}%</span></div>
                    <div class="metric-label">Transactions: {}</div>
                </div>
                <div class="stat-item">
                    <div class="metric-value">2-Hour</div>
                    <div class="metric-label">Final Value: <span class="{}">${:.2}</span></div>
                    <div class="metric-label">Return: <span class="{}">{:+.2}%</span></div>
                    <div class="metric-label">Transactions: {}</div>
                </div>
            </div>
        </div>

        <h2>⚠️ Fatal Decisions Analysis</h2>
        <div class="fatal-section">
            <p>Fatal decisions are those where the LLM made a buy decision but the price dropped by more than 3%, or made a sell decision but the price increased by more than 3%.</p>
            <div class="metrics-grid">
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Buy Decisions (30m)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Buy Decisions (1h)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Buy Decisions (2h)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Sell Decisions (30m)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Sell Decisions (1h)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value fatal">{}</div>
                    <div class="metric-label">Fatal Sell Decisions (2h)</div>
                </div>
            </div>
        </div>

        <h2>📊 Performance Metrics</h2>
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value">{:.1}%</div>
                <div class="metric-label">Overall Accuracy (30m)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.1}%</div>
                <div class="metric-label">Overall Accuracy (1h)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.1}%</div>
                <div class="metric-label">Overall Accuracy (2h)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value {}">{:+.2}%</div>
                <div class="metric-label">Total P&L (30m)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value {}">{:+.2}%</div>
                <div class="metric-label">Total P&L (1h)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value {}">{:+.2}%</div>
                <div class="metric-label">Total P&L (2h)</div>
            </div>
        </div>

        <h2>📈 Detailed Decision Results</h2>
        <table>
            <thead>
                <tr>
                    <th>Timestamp</th>
                    <th>Price</th>
                    <th>Action</th>
                    <th>Confidence</th>
                    <th>30m P&L</th>
                    <th>1h P&L</th>
                    <th>2h P&L</th>
                    <th>30m Result</th>
                    <th>1h Result</th>
                    <th>2h Result</th>
                    <th>Reasoning</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>

        <h2>💼 Transaction History</h2>
        <h3>30-Minute Strategy</h3>
        <table>
            <thead>
                <tr><th>Timestamp</th><th>Action</th><th>Price</th><th>Amount</th><th>Value</th><th>Portfolio After</th></tr>
            </thead>
            <tbody>{}</tbody>
        </table>

        <h3>1-Hour Strategy</h3>
        <table>
            <thead>
                <tr><th>Timestamp</th><th>Action</th><th>Price</th><th>Amount</th><th>Value</th><th>Portfolio After</th></tr>
            </thead>
            <tbody>{}</tbody>
        </table>

        <h3>2-Hour Strategy</h3>
        <table>
            <thead>
                <tr><th>Timestamp</th><th>Action</th><th>Price</th><th>Amount</th><th>Value</th><th>Portfolio After</th></tr>
            </thead>
            <tbody>{}</tbody>
        </table>

        <div style="margin-top: 40px; padding-top: 20px; border-top: 2px solid #ecf0f1; text-align: center; color: #7f8c8d;">
            <p>Report generated by BotMarley Trading Analysis System</p>
        </div>
    </div>
</body>
</html>
"#,
        timestamp,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        report.symbol,
        report.model,
        report.test_period_from,
        report.test_period_to,
        report.config.openai_base_url,
        report.total_decisions,
        report.initial_portfolio_value,
        
        // 30m portfolio
        if report.portfolio_return_30m >= 0.0 { "positive" } else { "negative" },
        report.final_portfolio_value_30m,
        if report.portfolio_return_30m >= 0.0 { "positive" } else { "negative" },
        report.portfolio_return_30m,
        sim_30m.transactions.len(),
        
        // 1h portfolio
        if report.portfolio_return_1h >= 0.0 { "positive" } else { "negative" },
        report.final_portfolio_value_1h,
        if report.portfolio_return_1h >= 0.0 { "positive" } else { "negative" },
        report.portfolio_return_1h,
        sim_1h.transactions.len(),
        
        // 2h portfolio
        if report.portfolio_return_2h >= 0.0 { "positive" } else { "negative" },
        report.final_portfolio_value_2h,
        if report.portfolio_return_2h >= 0.0 { "positive" } else { "negative" },
        report.portfolio_return_2h,
        sim_2h.transactions.len(),
        
        // Fatal decisions
        report.fatal_buy_decisions_30m,
        report.fatal_buy_decisions_1h,
        report.fatal_buy_decisions_2h,
        report.fatal_sell_decisions_30m,
        report.fatal_sell_decisions_1h,
        report.fatal_sell_decisions_2h,
        
        // Accuracy metrics
        report.accuracy_30m,
        report.accuracy_1h,
        report.accuracy_2h,
        
        // P&L metrics
        if report.total_profit_loss_30m >= 0.0 { "positive" } else { "negative" },
        report.total_profit_loss_30m,
        if report.total_profit_loss_1h >= 0.0 { "positive" } else { "negative" },
        report.total_profit_loss_1h,
        if report.total_profit_loss_2h >= 0.0 { "positive" } else { "negative" },
        report.total_profit_loss_2h,
        
        // Decision results table
        generate_decision_table_rows(&report.results),
        
        // Transaction tables
        generate_transaction_table_rows(&sim_30m.transactions),
        generate_transaction_table_rows(&sim_1h.transactions),
        generate_transaction_table_rows(&sim_2h.transactions),
    );
    
    std::fs::write(&filename, html_content)?;
    println!("📄 HTML report generated: {}", filename);
    Ok(())
}

fn generate_decision_table_rows(results: &[DecisionResult]) -> String {
    results.iter().map(|result| {
        let timestamp = DateTime::<Utc>::from_timestamp_millis(result.timestamp)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S");
        
        format!(
            "<tr class={}>
                <td>{}</td>
                <td>${:.2}</td>
                <td>{}</td>
                <td>{:.0}%</td>
                <td class=\"{}\">{:+.2}%</td>
                <td class=\"{}\">{:+.2}%</td>
                <td class=\"{}\">{:+.2}%</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td style=\"max-width: 200px; overflow: hidden; text-overflow: ellipsis;\">{}</td>
            </tr>",
            result.decision.action.to_lowercase(),
            timestamp,
            result.price,
            result.decision.action.to_uppercase(),
            result.decision.confidence * 100.0,
            if result.profit_loss_30m.unwrap_or(0.0) >= 0.0 { "positive" } else { "negative" },
            result.profit_loss_30m.unwrap_or(0.0),
            if result.profit_loss_1h.unwrap_or(0.0) >= 0.0 { "positive" } else { "negative" },
            result.profit_loss_1h.unwrap_or(0.0),
            if result.profit_loss_2h.unwrap_or(0.0) >= 0.0 { "positive" } else { "negative" },
            result.profit_loss_2h.unwrap_or(0.0),
            if result.was_correct_30m.unwrap_or(false) { "✅" } else { "❌" },
            if result.was_correct_1h.unwrap_or(false) { "✅" } else { "❌" },
            if result.was_correct_2h.unwrap_or(false) { "✅" } else { "❌" },
            result.decision.reasoning.chars().take(100).collect::<String>()
        )
    }).collect::<Vec<_>>().join("\n")
}

fn generate_transaction_table_rows(transactions: &[Transaction]) -> String {
    transactions.iter().map(|tx| {
        let timestamp = DateTime::<Utc>::from_timestamp_millis(tx.timestamp)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S");
        
        format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>${:.2}</td>
                <td>{:.6}</td>
                <td>${:.2}</td>
                <td>${:.2}</td>
            </tr>",
            timestamp,
            tx.action.to_uppercase(),
            tx.price,
            tx.amount,
            tx.value,
            tx.portfolio_value_after
        )
    }).collect::<Vec<_>>().join("\n")
}
