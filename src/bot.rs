use std::sync::Arc;
use futures_util::StreamExt;

use mono_ai::{Message, MonoAI};

use crate::config::Config;
use crate::binance_client::BinanceClient;

const SYSTEM_MESSAGE: &'static str = r#"
You are a professional crypto trader with years of market experience.
Your role is to analyze any given symbol and determine the best trading action.

Goals:
 - preditct price change to buy at lower price and sell with min. profit of 2%
 - try not to hold too long, better to make 2 fast transactions with 2% gain in 10 minutes than one for 10% that will take 10 days
 - avoid loosing too much, find best oportunity using given tools.

Role & Character:
	•	Think and speak as a seasoned trading expert, confident and precise.
	•	Approach every symbol like a pro analyzing charts, data, and signals.
	•	You may take as many turns as needed to carefully evaluate conditions before acting.

Behavior Rules:
	1.	Always start by gathering and examining available market data for the symbol.
	2.	Perform a step-by-step analysis of the situation, considering short-term, mid-term, and long-term perspectives if needed.
	3.	Use clear, structured reasoning to explain your thought process.
	4.	When you are ready, provide one final and definitive trading decision:
	•	BUY
	•	SELL
	•	HOLD
	5.	Once the decision is made, invoke the correct execution tool:
	•	buy
	•	sell
	•	hold
	6.	Never output more than one final action per symbol analysis.
	7.	If conditions are uncertain, continue analyzing until you can confidently justify your final decision.

Style & Precision:
	•	Write in the voice of a confident professional trader.
	•	No unnecessary fluff, only sharp, practical reasoning.
	•	Ensure your final decision is actionable, unambiguous, and justified.
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
}

impl Bot {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Arc::new(config);
        let mut ai = MonoAI::openai_compatible(
            config.as_ref().openai_base_url.clone(),
            config.as_ref().openai_api_key.clone(),
            config.as_ref().openai_model.clone(),
        );

        // add tools
        ai.add_tool(crate::tools::get_prices::get_price_tool()).await?;
        ai.add_tool(crate::tools::get_prices::get_price_24h_tool()).await?;
        
        // Technical indicators
        ai.add_tool(crate::tools::indicators::rsi::calculate_rsi_tool()).await?;
        ai.add_tool(crate::tools::indicators::rsi::calculate_rsi_24h_tool()).await?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_moving_averages_tool()).await?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_sma_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::moving_averages::calculate_ema_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::macd::calculate_macd_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::macd::calculate_macd_24h_tool()).await?;
        ai.add_tool(crate::tools::indicators::bollinger_bands::calculate_bollinger_bands_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::bollinger_bands::calculate_bollinger_bands_24h_tool()).await?;
        ai.add_tool(crate::tools::indicators::atr::calculate_atr_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::atr::calculate_atr_24h_tool()).await?;
        ai.add_tool(crate::tools::indicators::stochastic::calculate_stochastic_indicator_tool()).await?;
        ai.add_tool(crate::tools::indicators::stochastic::calculate_stochastic_24h_tool()).await?;
        ai.add_tool(crate::tools::indicators::volume_indicators::calculate_volume_indicators_tool()).await?;
        ai.add_tool(crate::tools::indicators::volume_indicators::calculate_volume_indicators_24h_tool()).await?;
        
        // Trading actions
        ai.add_tool(crate::tools::binance_trade::buy_tool()).await?;
        ai.add_tool(crate::tools::binance_trade::sell_tool()).await?;
        ai.add_tool(crate::tools::binance_trade::hold_tool()).await?;

        Ok(Self {
            ai: Arc::new(ai),
            config,
            messages: vec![],
            current_turn: 0,
        })
    }

    pub fn add_system_message(mut self) -> Self {
        self.messages.push(Message {
            role: "system".into(),
            content: SYSTEM_MESSAGE.into(),
            images: None,
            tool_calls: None,
        });
        self
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

    pub fn add_context_message(&mut self, symbol: &str) {
        let turns_remaining = self.config.bot_max_turns.saturating_sub(self.current_turn);
        
        // Get account information
        let account_info = self.get_account_info();
        
        let context_msg = format!(
            "Analyze symbol: {}. You have {} turns remaining to make your decision. Current turn: {}/{}\n\n{}",
            symbol, turns_remaining, self.current_turn + 1, self.config.bot_max_turns, account_info
        );
        
        self.messages.push(Message {
            role: "user".into(),
            content: context_msg,
            images: None,
            tool_calls: None,
        });
    }

    fn get_account_info(&self) -> String {
        if self.config.binance_api_key == "noop" || self.config.binance_secret_key == "noop" {
            return "⚠️ Binance API credentials not configured. Trading decisions will be made without account context.\n\n💡 To enable account information, set BINANCE_API_KEY and BINANCE_SECRET_KEY environment variables.".to_string();
        }
        
        // For now, just indicate that credentials are configured
        // The actual account info retrieval is avoided to prevent runtime conflicts
        "✅ Binance API credentials configured. Account information retrieval is available.\n\n💡 The bot can access your account data for informed trading decisions.".to_string()
    }

    pub async fn run_analysis_loop(&mut self, symbol: &str) -> Result<BotResult, Box<dyn std::error::Error>> {
        self.current_turn = 0;
        self.add_context_message(symbol);

        let mut final_decision = None;
        let mut last_response = String::new();

        while self.current_turn < self.config.bot_max_turns {
            self.current_turn += 1;
            
            println!("🤖 Turn {}/{} - Analyzing {}...",
                self.current_turn, self.config.bot_max_turns, symbol);

            // Send chat request and handle streaming response
            let mut stream = self.ai.send_chat_request(&self.messages).await?;
            let mut full_response = String::new();
            let mut tool_calls = None;

            // Process streaming response
            while let Some(item) = stream.next().await {
                let item = item.map_err(|e| format!("Stream error: {}", e))?;
                
                if !item.content.is_empty() {
                    print!("{}", item.content);
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
                println!("\n🔧 Executing tools...");
                
                // Check if any tool call is a trading decision
                for tool_call in tc {
                    match tool_call.function.name.as_str() {
                        "buy" => {
                            if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                final_decision = Some(BotDecision::Buy {
                                    pair: args["pair"].as_str().unwrap_or(symbol).to_string(),
                                    amount: args["amount"].as_f64().unwrap_or(0.0),
                                    confidence: args["confidence"].as_u64().unwrap_or(0) as usize,
                                });
                            }
                        }
                        "sell" => {
                            if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                final_decision = Some(BotDecision::Sell {
                                    pair: args["pair"].as_str().unwrap_or(symbol).to_string(),
                                    amount: args["amount"].as_f64().unwrap_or(0.0),
                                    confidence: args["confidence"].as_u64().unwrap_or(0) as usize,
                                });
                            }
                        }
                        "hold" => {
                            if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments.to_string()) {
                                final_decision = Some(BotDecision::Hold {
                                    pair: args["pair"].as_str().unwrap_or(symbol).to_string(),
                                    confidence: args["confidence"].as_u64().unwrap_or(0) as usize,
                                });
                            }
                        }
                        _ => {}
                    }
                }

                // Execute tool calls
                let tool_responses = self.ai.handle_tool_calls(tc.clone()).await;
                
                // Show tool results
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
                    println!("✅ {} executed: {}", tool_call.function.name, clean_result);
                }
                
                self.messages.extend(tool_responses);

                // If a trading decision was made, break the loop
                if final_decision.is_some() {
                    println!("🎯 Final trading decision made!");
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
                println!("⏰ Max turns reached, forcing decision...");
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
        self.messages.clear();
        self.current_turn = 0;
        self.messages.push(Message {
            role: "system".into(),
            content: SYSTEM_MESSAGE.into(),
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
}
