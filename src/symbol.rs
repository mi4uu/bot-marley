use std::str::FromStr;

use serde::{Deserialize, Serialize};


pub enum Asset {
    Btc,
    Usdc,
    Bnb,
    Etc,
    Ada,
    Dot,
    Doge,
    Mito,
    Zen,
    Tut,
    Eigen,
    Ethfi,
    Floki
}
// near? grt

#[derive(Debug,Clone,PartialEq, Eq,Serialize, Deserialize)]
pub struct Symbol {
     base:String,
     quote:String,
}
impl Symbol {
    pub fn new(base:String, quote:String)->Self{
        let base=base.to_uppercase();
        let quote=quote.to_uppercase();
        let quote=match quote.as_str(){
            "BTC"=>"BTC".to_string(),
            "USDC"=>"USDC".to_string(),
            _=> panic!("QUOTE NOT ALLOWED")
        };
        Self{
            base,quote
        }
    }
    pub fn get_quote(&self)->String{
        self.quote.clone()
    }
      pub fn get_base(&self)->String{
        self.base.clone()
    }
}

impl From<String> for Symbol{
fn from(value: String) -> Self {
    let value=value.to_uppercase().replace("_", "");

   if value.ends_with("BTC"){
    return Symbol { base: value.rsplit_once("BTC").unwrap().0.to_string(), quote: "BTC".into() }
   }
 if value.ends_with("USDC"){
    return Symbol { base: value.rsplit_once("USDC").unwrap().0.to_string(), quote: "USDC".into() }
   }

panic!("cannot convert to symbol");
}
}
impl From<&str> for Symbol{
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl ToString for Symbol{
    fn to_string(&self) -> String {
        format!("{}{}",self.base,self.quote)
    }
}