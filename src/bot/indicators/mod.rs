use serde::{Deserialize, Serialize};
pub mod macd;

#[derive(Debug,Serialize,Deserialize)]
pub struct Indicator{
   pub name: String,
   pub  description: String,
   pub symbol:String,
    
}

// pub trait indidator{
    
// }