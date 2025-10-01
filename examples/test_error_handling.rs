use color_eyre::eyre::{Result, eyre, WrapErr};
use color_eyre::Section;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;

    println!("🧪 Testing color-eyre error handling...");

    // Test 1: Basic error with context
    test_basic_error().await?;

    // Test 2: Error with suggestions
    test_error_with_suggestions().await?;

    // Test 3: Nested error handling
    test_nested_errors().await?;

    println!("✅ All error handling tests completed!");
    Ok(())
}

async fn test_basic_error() -> Result<()> {
    println!("\n📋 Test 1: Basic error with context");
    
    // Simulate a file operation that fails
    std::fs::read_to_string("nonexistent_file.txt")
        .wrap_err("Failed to read configuration file")
        .map_err(|e| e.with_suggestion(|| "Make sure the file exists and is readable"))?;
    
    Ok(())
}

async fn test_error_with_suggestions() -> Result<()> {
    println!("\n📋 Test 2: Error with suggestions");
    
    // Simulate a network connection error
    Err(eyre!("Connection refused"))
        .wrap_err("Failed to connect to trading API")
        .with_suggestion(|| "Check your internet connection")
        .with_suggestion(|| "Verify API credentials are correct")
        .with_suggestion(|| "Check if the API service is running")?;
    
    Ok(())
}

async fn test_nested_errors() -> Result<()> {
    println!("\n📋 Test 3: Nested error handling");
    
    // Simulate a complex operation with multiple failure points
    simulate_trading_operation()
        .await
        .wrap_err("Trading operation failed")
        .with_suggestion(|| "Review your trading strategy")
        .with_suggestion(|| "Check market conditions")?;
    
    Ok(())
}

async fn simulate_trading_operation() -> Result<()> {
    // Simulate authentication failure
    authenticate_user()
        .wrap_err("Authentication step failed")?;
    
    // Simulate market data fetch failure
    fetch_market_data()
        .await
        .wrap_err("Market data retrieval failed")?;
    
    Ok(())
}

fn authenticate_user() -> Result<()> {
    Err(eyre!("Invalid API key"))
        .with_suggestion(|| "Check your API key in the .env file")
}

async fn fetch_market_data() -> Result<()> {
    Err(eyre!("Market data service unavailable"))
        .with_suggestion(|| "Try again in a few minutes")
        .with_suggestion(|| "Check the service status page")
}