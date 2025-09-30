use mono_ai_macros::tool;
use reqwest::blocking::Client;
use serde::Deserialize;
use futures_util::StreamExt;


struct Kline {
    // Note: The fields are in the order they appear in the Binance API response array.
    // We use a custom deserialization trait (if we were to implement it) or 
    // simply rely on accessing the array indices (as shown in the main function).
    // The actual response is an array of 12 elements.
    // [0] Open time
    // [1] Open price
    // [2] High price
    // [3] Low price
    // [4] Close price
    // [5] Volume
    // ... (other fields)
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    // ... you can add the remaining fields if needed
}



#[tool]
/// Get price of asset 
pub   fn get_price(symbol: String) -> String {
    
        let client = Client::new();

    let url = "https://api.binance.com/api/v3/klines";
let params = [
        ("symbol", symbol),
        ("interval", "5m".into()),
        ("limit", "5".into()), // Limit to 5 Klines for a brief example
    ];
      let response = client
        .get(url)
        .query(&params)
        .send()
        .expect("unable to get prices")
        .json::<serde_json::Value>() // Deserialize into Vec<Vec<Value>>
        .expect("unable to get prices");

  //  println!("Successfully fetched {} Klines:", response.len());

    
    response.to_string()
}
