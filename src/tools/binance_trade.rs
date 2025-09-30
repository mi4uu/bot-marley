use mono_ai_macros::tool;



#[tool]
/// Sell asset, confidence in % about this decision , THIS IS FINAL DECISION
fn sell(pair: String, amount:f64, confidence: usize, explanation: String) -> String {
    println!("---------------------------------------");

    println!("FINAL DECISION: SELL 💰");
    println!("CONFIDENCE: {} %", confidence);
    println!("EXPLANATION : {} ", explanation);
    println!("---------------------------------------");
    format!("Sold {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn buy(pair: String, amount:f64,confidence: usize, explanation: String) -> String {
    println!("---------------------------------------");

    println!("FINAL DECISION: BUY 🛍️");
    println!("CONFIDENCE: {} %", confidence);
    println!("EXPLANATION : {} ", explanation);
    println!("---------------------------------------");
    format!("Buy {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn hold(pair: String,confidence: usize, explanation: String) -> String {
    println!("---------------------------------------");

    println!("FINAL DECISION: HOLD ⌛");
    println!("CONFIDENCE: {} %", confidence);
    println!("EXPLANATION : {} ", explanation);
    println!("---------------------------------------");

    format!("Hold for pair {}",  pair)
}
