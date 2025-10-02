use std::sync::Arc;
use futures_util::StreamExt;
use tracing::{info, debug, warn, info_span};
use chrono::Utc;
use color_eyre::eyre::{Result, eyre};
use color_eyre::Section;

use mono_ai::{Message, MonoAI};

use crate::binance_client::BinanceClient;
use crate::config::Config;
use crate::persistence::{PersistenceManager, TradingState, TradingDecision};
use crate::tools::get_prices::fetch_klines_cached;

const SYSTEM_MESSAGE: &'static str = r#"
You are a professional crypto trader with years of market experience.
Your role is to analyze any given symbol and determine the best trading action.
Dont rust, take your time, gather all information and reason to understand context before making decision.

Goals:
 - preditct price change to buy at lower price and sell with min. profit of 2%
 - try not to hold too long, better to make 2 fast transactions with 2% gain in 10 minutes than one for 10% that will take 10 days
 - avoid loosing too much, if there is real risk of market colapsing you can protect assets as usdc.
 - find best oportunity using given tools to make a fast profit. 
 - manage portfolio to increase portfolio value fast and avoid value loss, understanding all markets conditions, other wallet assets and previous decisions and transactions.
 - take adventage of market swings, volotality, scalping .

Role & Character:
	•	Think and speak as a seasoned trading expert, confident and precise.
	•	Approach every symbol like a pro analyzing charts, data, and signals.
	•	You may take as many turns as needed to carefully evaluate conditions before acting.

Previous Decision Context:
	•	You will be provided with your previous trading decisions for each symbol.
	•	Consider your past decisions, their confidence levels, and explanations.
	•	Learn from previous patterns - if you consistently made certain decisions, analyze why.
	•	Avoid repeating the same mistakes or being overly conservative/aggressive based on past performance.
	•	Use historical context to improve decision quality, but don't be bound by past decisions if market conditions have changed.

Behavior Rules:
	1.	Always start by gathering and examining available market data for the symbol.
	2.	Review your previous trading history for this symbol if provided.
	3.	Perform a step-by-step analysis of the situation, considering short-term, mid-term, and long-term perspectives if needed.
	4.	Use clear, structured reasoning to explain your thought process.
	5.	When you are ready, provide one final and definitive trading decision:
	•	BUY
	•	SELL
	•	HOLD
	6.	Once the decision is made, invoke the correct execution tool:
	•	buy
	•	sell
	•	hold
	7.	Never output more than one final action per symbol analysis.
	8.	If conditions are uncertain, continue analyzing until you can confidently justify your final decision.

Style & Precision:
	•	Write in the voice of a confident professional trader.
	•	No unnecessary fluff, only sharp, practical reasoning.
	•	Ensure your final decision is actionable, unambiguous, and justified.
	•	Reference previous decisions when relevant to current analysis.
"#;

#[derive(Debug)]
pub enum BotDecision {
    Buy { pair: String, amount: f64, confidence: usize },
    Sell { pair: String, amount: f64, confidence: usize },
    Hold { pair: String, confidence: usize },
}

#[derive(Debug)]
pub struct BotResult {
    pub decision: Option<BotDecision>,
    pub turns_used: usize,
    pub final_response: String,
    pub conversation_history: Vec<Message>,
}

#[derive(Clone)]
pub struct Bot {
    ai: Arc<MonoAI>,
    config: Arc<Config>,
    messages: Vec<Message>,
    current_turn: usize,
    persistence_manager: Arc<PersistenceManager>,
    trading_state: TradingState,
}

impl Bot {
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let mut ai = MonoAI::openai_compatible(
            config.as_ref().openai_base_url.clone(),
            config.as_ref().openai_api_key.clone(),
            config.as_ref().openai_model.clone(),
        );

        // add tools
        // Add tools with proper error handling
        ai.add_tool(crate::tools::get_prices::get_price_tool()).await
            .map_err(|e| eyre!("Failed to add price tool: {}", e))?;
        ai.add_tool(crate::tools::get_prices::get_price_24h_tool()).await
            .map_err(|e| eyre!("Failed to add 24h price tool: {}", e))?;
        
        // Technical indicators
        ai.add_tool(crate::tools::indicators::rsi::calculate_rsi_tool()).await
            .map_err(|e| eyre!("Failed to add RSI tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::rsi::calculate_rsi_24h_tool()).await
            .map_err(|e| eyre!("Failed to add RSI 24h tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_moving_averages_tool()).await
            .map_err(|e| eyre!("Failed to add moving averages tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_sma_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add SMA tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_ema_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add EMA tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::macd::calculate_macd_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add MACD tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::macd::calculate_macd_24h_tool()).await
            .map_err(|e| eyre!("Failed to add MACD 24h tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::bollinger_bands::calculate_bollinger_bands_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add Bollinger Bands tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::bollinger_bands::calculate_bollinger_bands_24h_tool()).await
            .map_err(|e| eyre!("Failed to add Bollinger Bands 24h tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::atr::calculate_atr_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add ATR tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::atr::calculate_atr_24h_tool()).await
            .map_err(|e| eyre!("Failed to add ATR 24h tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::stochastic::calculate_stochastic_indicator_tool()).await
            .map_err(|e| eyre!("Failed to add Stochastic tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::stochastic::calculate_stochastic_24h_tool()).await
            .map_err(|e| eyre!("Failed to add Stochastic 24h tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::volume_indicators::calculate_volume_indicators_tool()).await
            .map_err(|e| eyre!("Failed to add volume indicators tool: {}", e))?;
        ai.add_tool(crate::tools::indicators::volume_indicators::calculate_volume_indicators_24h_tool()).await
            .map_err(|e| eyre!("Failed to add volume indicators 24h tool: {}", e))?;
        
        // Trading actions
        ai.add_tool(crate::tools::binance_trade::buy_tool()).await
            .map_err(|e| eyre!("Failed to add buy tool: {}", e))?;
        ai.add_tool(crate::tools::binance_trade::sell_tool()).await
            .map_err(|e| eyre!("Failed to add sell tool: {}", e))?;
        ai.add_tool(crate::tools::binance_trade::hold_tool()).await
            .map_err(|e| eyre!("Failed to add hold tool: {}", e))?;

        // Initialize persistence manager
        let persistence_manager = Arc::new(PersistenceManager::new("data/trading_state.json"));
        let trading_state = persistence_manager.load_state();
        
        Ok(Self {
            ai: Arc::new(ai),
            config,
            messages: vec![],
            current_turn: 0,
            persistence_manager,
            trading_state,
        })
    }

    pub fn get_system_message(mut self) -> String {
        let sell_limit:f64=self.config.max_trade_value as f64;
        let sell_limit=sell_limit*1.1;
        let restrictions = vec![
        format!("max buy for amount asset value equal to {} USDC",  self.config.max_trade_value),
         format!("max sell for amount asset value equal to {} USDC",  sell_limit),
        //   format!("max active orders: {}",  self.config.max_trade_value),
        //    format!("max buy for amount equal to {} usd",  self.config.max_trade_value),
        ];
        
          format!(r#"{}
        
RESTRICTIONS:
{}
        
ALL TRADING PAIRS: {}

        "#,SYSTEM_MESSAGE, restrictions.join("\n- "), self.config.as_ref().pairs().join(", "))
    }
    pub fn add_user_message(mut self, msg: String) -> Self {
        self.messages.push(Message {
            role: "user".into(),
            content: msg,
            images: None,
            tool_calls: None,
        });
        self
    }

    pub async fn add_context_message(&mut self, symbol: &str) {
        let turns_remaining = self.config.bot_max_turns.saturating_sub(self.current_turn);
        
        // Get account information
        let account_info = self.get_account_info().await;
        
        // Get trading history for this symbol
        let trading_history = self.trading_state.generate_context_summary(symbol);
        
        let context_msg = format!(
            "Analyze symbol: {}. You have {} turns remaining to make your decision. Current turn: {}/{}\n\n{}\n{}",
            symbol, turns_remaining, self.current_turn + 1, self.config.bot_max_turns, account_info, trading_history
        );
        
        self.messages.push(Message {
            role: "user".into(),
            content: context_msg,
            images: None,
            tool_calls: None,
        });
    }

    async fn get_account_info(&self) -> String {
        if self.config.binance_api_key == "noop" || self.config.binance_secret_key == "noop" {
            return "⚠️ Binance API credentials not configured. Trading decisions will be made without account context.\n\n💡 To enable account information, set BINANCE_API_KEY and BINANCE_SECRET_KEY environment variables.".to_string();
        }
        
        let api_key = self.config.binance_api_key.clone();
        let secret_key = self.config.binance_secret_key.clone();
        
        // Use spawn_blocking to run the blocking Binance client code in a separate thread
        match tokio::task::spawn_blocking(move || {
            BinanceClient::new(api_key, secret_key)
                .and_then(|client| client.format_account_summary())
        }).await {
            Ok(Ok(summary)) => summary,
            Ok(Err(e)) => format!("ERROR GETTING ACCOUNT SUMMARY: {}", e),
            Err(e) => format!("ERROR SPAWNING BLOCKING TASK: {}", e),
        }
    }

    /// Get the latest price timestamp for a symbol from Binance data
    async fn get_latest_price_timestamp(&self, symbol: &str) -> Result<Option<u64>> {
        // Fetch the latest kline data (just 1 candle to get the most recent timestamp)
        match fetch_klines_cached(symbol, "5m", 1).await {
            Ok(klines) => {
                if let Some(latest_kline) = klines.last() {
                    Ok(Some(latest_kline.close_time))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to fetch price timestamp for {}: {:?}", symbol, e);
                Ok(None)
            }
        }
    }

    pub async fn run_analysis_loop(&mut self, symbol: &str, extra_instructions:String) -> Result<BotResult> {
        // Check if we already made a decision for the current price timestamp
        if let Ok(Some(current_price_timestamp)) = self.get_latest_price_timestamp(symbol).await {
            if self.trading_state.has_decision_for_timestamp(symbol, current_price_timestamp) {
                info!("⏭️ Skipping analysis for {} - decision already made for timestamp {}", symbol, current_price_timestamp);
                return Ok(BotResult {
                    decision: None,
                    turns_used: 0,
                    final_response: format!("Skipped: Decision already made for current price timestamp {}", current_price_timestamp),
                    conversation_history: vec![],
                });
            } else {
                info!("🆕 New price data for {} - timestamp: {}", symbol, current_price_timestamp);
            }
        } else {
            warn!("⚠️ Could not get price timestamp for {}, proceeding with analysis", symbol);
        }

        self.current_turn = 0;
        self.add_context_message(symbol).await;

        let mut final_decision = None;
        let mut last_response = String::new();

        while self.current_turn < self.config.bot_max_turns {
            self.current_turn += 1;
            
            let _turn_span = info_span!("bot_turn",
                symbol = symbol,
                turn = self.current_turn,
                max_turns = self.config.bot_max_turns
            ).entered();
            
            info!("🤖 Turn {}/{} - Analyzing {}...",
                self.current_turn, self.config.bot_max_turns, symbol);

            // Send chat request and handle streaming response
            let mut stream = match self.ai.send_chat_request(&self.messages).await {
                Ok(stream) => stream,
                Err(e) => {
                    return Err(eyre!("Failed to send chat request to AI: {}", e)
                        .with_suggestion(|| "Check your AI API credentials and network connection"));
                }
            };
            let mut full_response = String::new();
            let mut tool_calls = None;

            // Process streaming response
            while let Some(item) = stream.next().await {
                let item = item.map_err(|e| eyre!("Stream error: {}", e))?;
                
                if !item.content.is_empty() {
                    debug!("AI response chunk: {}", item.content);
                    full_response.push_str(&item.content);
                }
                
                if let Some(tc) = item.tool_calls {
                    tool_calls = Some(tc);
                }
                
                if item.done {
                    break;
                }
            }

            // Add assistant response to conversation
            self.messages.push(Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
                images: None,
                tool_calls: tool_calls.clone(),
            });

            last_response = full_response;

            // Handle tool calls if present
            if let Some(ref tc) = tool_calls {
                debug!("\n🔧 Executing tools...");
                
                // Execute tool calls first
                let tool_responses = match self.ai.handle_tool_calls(tc.clone()).await {
                    responses => responses, // handle_tool_calls doesn't return Result, so just use the responses
                };
                
                // Check if any tool call was a successful trading decision
                for (tool_call, response) in tc.iter().zip(tool_responses.iter()) {
                    let clean_result = if response.content.starts_with("TOOL_RESULT:") {
                        let parts: Vec<&str> = response.content.splitn(3, ':').collect();
                        if parts.len() == 3 {
                            parts[2]
                        } else {
                            &response.content
                        }
                    } else {
                        &response.content
                    };
                    
                    info!("✅ {} executed: {}", tool_call.function.name, clean_result);
                    
                    // Only record decision if the tool execution was successful (starts with ✅)
                    if clean_result.starts_with("✅") {
                        match tool_call.function.name.as_str() {
                            "buy" => {
                                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                    let pair = args["pair"].as_str().unwrap_or(symbol).to_string();
                                    let amount = args["amount"].as_f64().unwrap_or(0.0);
                                    let confidence = args["confidence"].as_u64().unwrap_or(0) as usize;
                                    let explanation = args["explanation"].as_str().unwrap_or("").to_string();
                                    
                                    final_decision = Some(BotDecision::Buy {
                                        pair: pair.clone(),
                                        amount,
                                        confidence,
                                    });
                                    
                                    // Get current price timestamp for deduplication
                                    let price_timestamp = self.get_latest_price_timestamp(&pair).await.unwrap_or(None);
                                    
                                    // Save trading decision to persistence
                                    let trading_decision = TradingDecision {
                                        symbol: pair,
                                        action: "BUY".to_string(),
                                        amount: Some(amount),
                                        confidence,
                                        explanation,
                                        timestamp: Utc::now(),
                                        price_at_decision: None, // TODO: Get current price
                                        price_timestamp,
                                    };
                                    self.trading_state.add_decision(trading_decision);
                                }
                            }
                            "sell" => {
                                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                    let pair = args["pair"].as_str().unwrap_or(symbol).to_string();
                                    let amount = args["amount"].as_f64().unwrap_or(0.0);
                                    let confidence = args["confidence"].as_u64().unwrap_or(0) as usize;
                                    let explanation = args["explanation"].as_str().unwrap_or("").to_string();
                                    
                                    final_decision = Some(BotDecision::Sell {
                                        pair: pair.clone(),
                                        amount,
                                        confidence,
                                    });
                                    
                                    // Get current price timestamp for deduplication
                                    let price_timestamp = self.get_latest_price_timestamp(&pair).await.unwrap_or(None);
                                    
                                    // Save trading decision to persistence
                                    let trading_decision = TradingDecision {
                                        symbol: pair,
                                        action: "SELL".to_string(),
                                        amount: Some(amount),
                                        confidence,
                                        explanation,
                                        timestamp: Utc::now(),
                                        price_at_decision: None, // TODO: Get current price
                                        price_timestamp,
                                    };
                                    self.trading_state.add_decision(trading_decision);
                                }
                            }
                            "hold" => {
                                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                    let pair = args["pair"].as_str().unwrap_or(symbol).to_string();
                                    let confidence = args["confidence"].as_u64().unwrap_or(0) as usize;
                                    let explanation = args["explanation"].as_str().unwrap_or("").to_string();
                                    
                                    final_decision = Some(BotDecision::Hold {
                                        pair: pair.clone(),
                                        confidence,
                                    });
                                    
                                    // Get current price timestamp for deduplication
                                    let price_timestamp = self.get_latest_price_timestamp(&pair).await.unwrap_or(None);
                                    
                                    // Save trading decision to persistence
                                    let trading_decision = TradingDecision {
                                        symbol: pair,
                                        action: "HOLD".to_string(),
                                        amount: None,
                                        confidence,
                                        explanation,
                                        timestamp: Utc::now(),
                                        price_at_decision: None, // TODO: Get current price
                                        price_timestamp,
                                    };
                                    self.trading_state.add_decision(trading_decision);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                
                self.messages.extend(tool_responses);

                // Only break the loop if a successful trading decision was made
                if final_decision.is_some() {
                    info!("🎯 Final trading decision made!");
                    
                    // Save the updated trading state
                    if let Err(e) = self.persistence_manager.save_state(&self.trading_state) {
                        warn!("⚠️ Failed to save trading state: {}", e);
                    }
                    
                    break;
                }

                // Continue conversation after tool execution if no decision made
                if self.current_turn < self.config.bot_max_turns {
                    let turns_remaining = self.config.bot_max_turns - self.current_turn;
                    let follow_up = format!(
                        "Continue your analysis. {} turns remaining. Make your final decision when ready.",
                        turns_remaining
                    );
                    
                    self.messages.push(Message {
                        role: "user".to_string(),
                        content: follow_up,
                        images: None,
                        tool_calls: None,
                    });
                }
            } else if self.current_turn >= self.config.bot_max_turns {
                // Force a decision if max turns reached
                warn!("⏰ Max turns reached, forcing decision...");
                let force_decision_msg = format!(
                    "You have reached the maximum number of turns ({}). You must make a final trading decision NOW. Choose BUY, SELL, or HOLD for {} and execute the corresponding tool.",
                    self.config.bot_max_turns, symbol
                );
                
                self.messages.push(Message {
                    role: "user".to_string(),
                    content: force_decision_msg,
                    images: None,
                    tool_calls: None,
                });
            }
        }

        Ok(BotResult {
            decision: final_decision,
            turns_used: self.current_turn,
            final_response: last_response,
            conversation_history: self.messages.clone(),
        })
    }

    pub fn reset_conversation(&mut self) {
        let content = (self).clone().get_system_message().clone();
        self.messages.clear();
        self.current_turn = 0;
        self.messages.push(Message {
            role: "system".into(),
            content,
            images: None,
            tool_calls: None,
        });
    }

    pub fn get_conversation_history(&self) -> &Vec<Message> {
        &self.messages
    }

    pub fn get_current_turn(&self) -> usize {
        self.current_turn
    }

    pub fn get_max_turns(&self) -> usize {
        self.config.bot_max_turns
    }

    pub fn increment_run_counter(&mut self) {
        self.trading_state.increment_runs();
        if let Err(e) = self.persistence_manager.save_state(&self.trading_state) {
            warn!("⚠️ Failed to save run counter: {}", e);
        }
    }

    pub fn get_total_runs(&self) -> usize {
        self.trading_state.total_runs
    }
}
