# Binance Data Collector

This implementation provides a comprehensive data collection system for Binance cryptocurrency market data using Polars and Arrow file format for efficient storage and processing.

## Features

- **Incremental Data Collection**: Automatically detects the last timestamp in existing data files and fetches only new data
- **Arrow File Storage**: Uses Apache Arrow format for efficient columnar storage and fast querying
- **Rate Limiting**: Built-in rate limiting to respect Binance API limits
- **Error Handling**: Comprehensive error handling with detailed logging
- **Multi-pair Support**: Collects data for all configured trading pairs
- **5-minute Intervals**: Fetches OHLCV data at 5-minute intervals

## Configuration

The data collector uses the same configuration as the main bot. Make sure your `.env` file contains:

```env
BINANCE_API_KEY=your_api_key_here
BINANCE_SECRET_KEY=your_secret_key_here
ALLOWED_PAIRS=BNB_USDC,ETH_USDC,BTC_USDC,ADA_USDC,DOT_USDC,DOGE_USDC
```

## Usage

### Running the Data Collector

```bash
# Build the data collector
cargo build --bin data_collector

# Run the data collector
cargo run --bin data_collector
```

### Data Storage

Data is stored in the `data/` directory with the following structure:
- `data/bnbusdc_5m.arrow` - BNB/USDC 5-minute klines
- `data/ethusdc_5m.arrow` - ETH/USDC 5-minute klines
- `data/btcusdc_5m.arrow` - BTC/USDC 5-minute klines
- etc.

### Data Schema

Each Arrow file contains the following columns:
- `open_time`: Opening time (timestamp in milliseconds)
- `close_time`: Closing time (timestamp in milliseconds)
- `symbol`: Trading pair symbol
- `open`: Opening price
- `high`: Highest price
- `low`: Lowest price
- `close`: Closing price
- `volume`: Base asset volume
- `quote_asset_volume`: Quote asset volume
- `number_of_trades`: Number of trades
- `taker_buy_base_asset_volume`: Taker buy base asset volume
- `taker_buy_quote_asset_volume`: Taker buy quote asset volume

## Implementation Details

### Key Components

1. **DataCollector**: Main struct that handles API communication and data management
2. **KlineData**: Data structure representing a single kline (candlestick)
3. **Arrow File Management**: Functions for reading, writing, and appending to Arrow files
4. **Timestamp Tracking**: Logic to find the last timestamp and fetch incremental updates

### API Integration

- Uses `binance_rs_plus` crate for Binance API communication
- Implements proper rate limiting (100ms between requests)
- Handles pagination for large data sets (1000 klines per request)
- Supports both initial data collection and incremental updates

### Data Processing

- Uses Polars DataFrame for efficient data manipulation
- Automatic deduplication based on `open_time`
- Sorted storage for optimal query performance
- Columnar storage format for fast analytics

## Example Usage in Code

```rust
use botmarley::binance::DataCollector;
use botmarley::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let collector = DataCollector::new(config)?;
    
    // Collect data for all configured pairs
    collector.collect_all_data().await?;
    
    // Get statistics
    let stats = collector.get_data_stats()?;
    for (symbol, (count, min_time, max_time)) in stats {
        println!("{}: {} records from {:?} to {:?}", 
                 symbol, count, min_time, max_time);
    }
    
    Ok(())
}
```

## Performance Considerations

- **Memory Efficient**: Streams data in batches to avoid memory issues
- **Fast Storage**: Arrow format provides excellent compression and query performance
- **Incremental Updates**: Only fetches new data, reducing API calls and processing time
- **Concurrent Safe**: Can be run multiple times safely due to deduplication logic

## Error Handling

The implementation includes comprehensive error handling for:
- Network connectivity issues
- API rate limiting
- File I/O errors
- Data parsing errors
- Invalid timestamps

## Monitoring

The data collector provides detailed logging including:
- Progress updates for each trading pair
- Statistics on data fetched
- Error reporting with context
- Performance metrics

## Automation

You can set up the data collector to run periodically using:
- Cron jobs on Linux/macOS
- Task Scheduler on Windows
- Docker containers with scheduled execution
- CI/CD pipelines for continuous data collection

## Dependencies

- `polars`: DataFrame library with Arrow support
- `binance_rs_plus`: Binance API client
- `chrono`: Date and time handling
- `tokio`: Async runtime
- `tracing`: Structured logging
- `serde`: Serialization/deserialization