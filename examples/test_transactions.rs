use botmarley::persistence::TradingState;

fn main() {
    println!("Testing transaction parsing...");
    
    // Test reading transactions for ETHUSDC
    let transactions = TradingState::get_last_transactions_for_symbol("ETHUSDC", 5);
    println!("Found {} transactions for ETHUSDC:", transactions.len());
    
    for (i, transaction) in transactions.iter().enumerate() {
        println!("  {}. {} {} {:.4} @ ${:.4} = ${:.2}", 
            i + 1,
            transaction.transaction_type,
            transaction.asset,
            transaction.amount,
            transaction.price_per_unit,
            transaction.amount_in_usd
        );
    }
    
    // Test reading transactions for DOGEUSDC
    let transactions = TradingState::get_last_transactions_for_symbol("DOGEUSDC", 5);
    println!("\nFound {} transactions for DOGEUSDC:", transactions.len());
    
    for (i, transaction) in transactions.iter().enumerate() {
        println!("  {}. {} {} {:.4} @ ${:.4} = ${:.2}", 
            i + 1,
            transaction.transaction_type,
            transaction.asset,
            transaction.amount,
            transaction.price_per_unit,
            transaction.amount_in_usd
        );
    }
    
    // Test context summary generation
    let state = TradingState::new();
    let context = state.generate_context_summary("ETHUSDC");
    println!("\nContext summary for ETHUSDC:");
    println!("{}", context);
}