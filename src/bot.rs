use std::sync::Arc;

use mono_ai::{Message, MonoAI};

use crate::config::Config;



const SYSTEM_MESSAGE:&'static str = r#"
You are a professional crypto trader with years of market experience.
Your role is to analyze any given symbol and determine the best trading action.

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

#[derive(Clone)]
struct Bot{
    ai:Arc<MonoAI>,
    config: Arc<Config>,
    messages: Vec<Message>
}
impl Bot {
    pub async fn new(config:Config)->Result<Self, Box<dyn std::error::Error>>{
        let config = Arc::new(config);
        let mut ai = MonoAI::openai_compatible(config.as_ref().openai_base_url.clone(), config.as_ref().openai_api_key.clone(), config.as_ref().openai_model.clone());
    
        // add tools
        ai.add_tool(crate::tools::get_prices::get_price_tool()).await?;
 ai.add_tool(crate::tools::binance_trade::buy_tool()).await?;
  ai.add_tool(crate::tools::binance_trade::sell_tool()).await?;
 ai.add_tool(crate::tools::binance_trade::hold_tool()).await?;


        Ok(Self { ai: Arc::new(ai), config,messages:vec![] })
    }
      pub  fn add_system_message(mut self)->Self{

        self.messages.push(Message { role: "system".into(), content: SYSTEM_MESSAGE.into(), images: None, tool_calls: None });
        self
    }
    pub  fn add_user_message(mut self, msg:String)->Self{

        self.messages.push(Message { role: "user".into(), content: msg, images: None, tool_calls: None });
        self
    }

}
