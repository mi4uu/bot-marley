#[cfg(test)]
mod tests {
    
    use crate::persistence::{PersistenceManager, TradingState, TradingDecision};
    use chrono::Utc;
    use std::fs;

    #[test]
    fn test_persistence_system() {
        let test_file = "test_trading_state.json";
        
        // Clean up any existing test file
        let _ = fs::remove_file(test_file);
        
        // Create persistence manager
        let persistence_manager = PersistenceManager::new(test_file);
        
        // Create initial state
        let mut state = TradingState::new();
        
        // Add some test decisions
        let decision1 = TradingDecision {
            symbol: "BTCUSDT".to_string(),
            action: "BUY".to_string(),
            amount: Some(100.0),
            confidence: 85,
            explanation: "Strong bullish signal detected".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(45000.0),
            price_timestamp: Some(1640995499999),
        };
        
        let decision2 = TradingDecision {
            symbol: "ETHUSDT".to_string(),
            action: "HOLD".to_string(),
            amount: None,
            confidence: 70,
            explanation: "Waiting for better entry point".to_string(),
            timestamp: Utc::now(),
            price_at_decision: Some(3000.0),
            price_timestamp: Some(1640995799999),
        };
        
        state.add_decision(decision1);
        state.add_decision(decision2);
        state.increment_runs();
        
        // Save state
        persistence_manager.save_state(&state).expect("Failed to save state");
        
        // Load state
        let loaded_state = persistence_manager.load_state();
        
        // Verify loaded state
        assert_eq!(loaded_state.symbols.len(), 2);
        assert_eq!(loaded_state.total_runs, 1);
        
        // Check BTCUSDT history
        let btc_history = loaded_state.get_symbol_history("BTCUSDT").unwrap();
        assert_eq!(btc_history.buy_count, 1);
        assert_eq!(btc_history.sell_count, 0);
        assert_eq!(btc_history.hold_count, 0);
        assert_eq!(btc_history.total_decisions, 1);
        
        // Check ETHUSDT history
        let eth_history = loaded_state.get_symbol_history("ETHUSDT").unwrap();
        assert_eq!(eth_history.buy_count, 0);
        assert_eq!(eth_history.sell_count, 0);
        assert_eq!(eth_history.hold_count, 1);
        assert_eq!(eth_history.total_decisions, 1);
        
        // Test context summary generation
        let btc_summary = loaded_state.generate_context_summary("BTCUSDT");
        assert!(btc_summary.contains("BTCUSDT"));
        assert!(btc_summary.contains("BUY"));
        assert!(btc_summary.contains("85%"));
        
        let eth_summary = loaded_state.generate_context_summary("ETHUSDT");
        assert!(eth_summary.contains("ETHUSDT"));
        assert!(eth_summary.contains("HOLD"));
        assert!(eth_summary.contains("70%"));
        
        // Test unknown symbol
        let unknown_summary = loaded_state.generate_context_summary("ADAUSDT");
        assert!(unknown_summary.contains("No previous decisions found"));
        
        // Clean up test file
        let _ = fs::remove_file(test_file);
        
        println!("✅ All persistence tests passed!");
    }
}