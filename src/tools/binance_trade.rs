use mono_ai_macros::tool;



#[tool]
/// Sell asset, confidence in % about this decision , THIS IS FINAL DECISION
fn sell(pair: String, amount:f64, confidence: usize) -> String {
        print!("FINAL DECISION CONFIDENCE: {} %", confidence);

    format!("Sold {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn buy(pair: String, amount:f64,confidence: usize) -> String {
        print!("FINAL DECISION CONFIDENCE: {} %", confidence);

    format!("Buy {} for pair {}", amount, pair)
}
#[tool]
/// Buy asset, confidence in % about this decision, THIS IS FINAL DECISION
fn hold(pair: String,confidence: usize) -> String {
    print!("FINAL DECISION CONFIDENCE: {} %", confidence);
    format!("Hold for pair {}",  pair)
}
