
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;


#[derive(Debug,Serialize,Deserialize,SmartDefault)]
#[serde(default)]
pub struct Config{
    #[default = "noop"]
 pub   binance_api_key:String,
    #[default = "noop"]
 pub    binance_secret_key:String,
    #[default = "http://localhost:1234"]
  pub  openai_base_url:String,
    #[default = "noop"]
pub    openai_api_key:String,
    #[default = "openai/gpt-oss-20b"]
 pub   openai_model:String,
    #[default = 2]
  pub  max_active_orders:usize,
    #[default = 20]
pub    max_trade_value:usize,
    #[default = 3050]
 pub   web_ui_port:usize,
    allowed_pairs:String,
    #[default = "5m"]
   pub trading_interval:String,
   #[default = 30]
   pub bot_max_turns:usize

}

impl Config{
    pub fn load()->Self{
        envy::from_env::<Config>().unwrap_or_default()
    }
    pub fn pairs_parts(&self)->Vec<(String,String)>{
        let pairs:Vec<&str>=self.allowed_pairs.split(',').collect();
     let pairs_parts:Vec<(String,String)>=   pairs.into_iter().map(|p| { let parts=p.split('_').collect::<Vec<&str>>(); (parts[0].to_string(),parts[1].to_string()) } ).collect();
     pairs_parts
    }
    pub fn pairs(&self)->Vec<String>{
        self.pairs_parts().into_iter().map(|(l,r) | format!("{l}{r}")).collect()
    }
    pub 
fn symbols(&self) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    
    for (l, r) in self.pairs_parts().into_iter() {
        seen.insert(l);
        seen.insert(r);
    }
    
    seen.into_iter().collect()
}
}
