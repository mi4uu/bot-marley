// Required dependencies in Cargo.toml:
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// reqwest = { version = "0.11", features = ["json", "blocking"] }
// serde = { version = "1.0", features = ["derive"] }
// regex = "1.10"
// chrono = { version = "0.4", features = ["serde"] }

use reqwest;

const COINDESK_RSS_URL: &str = "https://www.coindesk.com/arc/outboundfeeds/rss";

#[derive(Debug)]
struct NewsItem {
    timestamp: String,
    sentiment: String,
    title: String,
    description: String,
    assets: Vec<String>,
}

impl NewsItem {
    fn format(&self) -> String {
        let mut result = format!("- {} [{}]", self.timestamp, self.sentiment);
        
        if !self.assets.is_empty() {
            result.push_str(&format!(" [{}]", self.assets.join(", ")));
        }
        
        result.push_str(&format!("\n    - {}", self.title));
        
        if !self.description.is_empty() && self.description.len() > 20 {
            // Truncate description to first sentence or 150 chars
            let desc = if self.description.len() > 150 {
                format!("{}...", &self.description[..147])
            } else {
                self.description.clone()
            };
            result.push_str(&format!("\n    - {}", desc));
        }
        
        result
    }
}

/// Fetch crypto news from CoinDesk RSS feed
pub async fn fetch_crypto_news() -> Result<String, Box<dyn std::error::Error>> {
    println!("Fetching content from: {}", COINDESK_RSS_URL);

    let client = reqwest::Client::new();
    let response = client
        .get(COINDESK_RSS_URL)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch RSS feed. Status: {}", response.status()).into());
    }

    let body = response.text().await?;
    println!("Successfully downloaded RSS feed.");
    

    parse_coindesk_rss(&body)
}

fn parse_coindesk_rss(xml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut news_items = Vec::new();
    
    // Extract items between <item> tags (case insensitive and multiline)
    let item_pattern = regex::Regex::new(r"(?is)<item[^>]*>(.*?)</item>")?;
    let title_pattern = regex::Regex::new(r"(?is)<title[^>]*>(?:<!\[CDATA\[(.*?)\]\]>|(.*?))</title>")?;
    let desc_pattern = regex::Regex::new(r"(?is)<description[^>]*>(?:<!\[CDATA\[(.*?)\]\]>|(.*?))</description>")?;
    let pubdate_pattern = regex::Regex::new(r"(?is)<pubDate[^>]*>(.*?)</pubDate>")?;

    let items: Vec<_> = item_pattern.find_iter(xml).collect();
    println!("Found {} items in RSS feed", items.len());

    for (i, item_match) in items.iter().enumerate().take(10) {
        let item_content = item_match.as_str();
        
        // Extract title
        let title = if let Some(caps) = title_pattern.captures(item_content) {
            caps.get(1).or(caps.get(2)).or(caps.get(3)).map(|m| clean_html(m.as_str())).unwrap_or("Untitled".to_string())
        } else {
            continue;
        };

        // Skip if title is too short or generic
        if title.len() < 10 || title.to_lowercase().contains("coindesk") {
            continue;
        }

        // Extract description
        let description = if let Some(caps) = desc_pattern.captures(item_content) {
            caps.get(1).or(caps.get(2)).or(caps.get(3)).map(|m| clean_html(m.as_str())).unwrap_or_default()
        } else {
            String::new()
        };

        // Extract and format timestamp
        let timestamp = if let Some(caps) = pubdate_pattern.captures(item_content) {
            if let Some(pubdate_str) = caps.get(1) {
                parse_rss_date(pubdate_str.as_str())
            } else {
                chrono::Local::now().format("%I:%M %p").to_string()
            }
        } else {
            chrono::Local::now().format("%I:%M %p").to_string()
        };

        let assets = extract_crypto_symbols(&format!("{} {}", title, description));
        let sentiment = analyze_sentiment(&title, &description);
        let truncated_description = if description.len() > 200 {
            format!("{}...", &description[..197])
        } else {
            description
        };

        let news_item = NewsItem {
            timestamp,
            sentiment,
            title,
            description: truncated_description,
            assets,
        };
        
        news_items.push(news_item);
    }

    if news_items.is_empty() {
        return Err("No news items found in RSS feed".into());
    }

    // Remove duplicates based on title
    let mut unique_items = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();
    
    for item in news_items {
        if !seen_titles.contains(&item.title) {
            seen_titles.insert(item.title.clone());
            unique_items.push(item);
        }
    }

    let mut output = String::new();
    for item in unique_items {
        output.push_str(&item.format());
        output.push('\n');
    }

    Ok(output.trim().to_string())
}

fn parse_rss_date(date_str: &str) -> String {
    // Try to parse RFC 2822 format (typical RSS format)
    // Example: "Wed, 02 Oct 2024 14:30:00 +0000"
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date_str) {
        return dt.with_timezone(&chrono::Local).format("%I:%M %p").to_string();
    }
    
    // Try RFC 3339 format
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return dt.with_timezone(&chrono::Local).format("%I:%M %p").to_string();
    }
    
    // Fallback to current time
    chrono::Local::now().format("%I:%M %p").to_string()
}

fn analyze_sentiment(title: &str, description: &str) -> String {
    let text = format!("{} {}", title, description).to_lowercase();
    
    // Positive indicators
    let positive_words = vec![
        "rally", "rallies", "surge", "surges", "gain", "gains", "rise", "rises", "boost", "boosts",
        "higher", "soar", "soars", "jump", "jumps", "bullish", "positive", "up", "increase", "increases",
        "growth", "strong", "breakthrough", "adoption", "partnership", "upgrade", "success", "milestone",
        "record", "high", "peak", "moon", "pump", "bull", "green", "profit", "win", "victory"
    ];
    
    // Negative indicators  
    let negative_words = vec![
        "fall", "falls", "drop", "drops", "decline", "declines", "crash", "crashes", "plunge", "plunges",
        "lower", "sink", "sinks", "tumble", "tumbles", "bearish", "negative", "down", "decrease", "decreases",
        "loss", "losses", "weak", "concern", "concerns", "regulation", "ban", "bans", "hack", "hacks",
        "vulnerability", "risk", "risks", "fear", "fears", "sell", "dump", "bear", "red", "blood"
    ];

    let positive_count = positive_words.iter().filter(|&word| text.contains(word)).count();
    let negative_count = negative_words.iter().filter(|&word| text.contains(word)).count();

    if positive_count > negative_count && positive_count > 0 {
        "Positive".to_string()
    } else if negative_count > positive_count && negative_count > 0 {
        "Negative".to_string()
    } else {
        "Neutral".to_string()
    }
}

fn extract_crypto_symbols(text: &str) -> Vec<String> {
    let text_upper = text.to_uppercase();
    
    let crypto_symbols = vec![
        ("BITCOIN", "BTC"), ("ETHEREUM", "ETH"), ("RIPPLE", "XRP"), ("XRP", "XRP"),
        ("CARDANO", "ADA"), ("ADA", "ADA"), ("POLKADOT", "DOT"), ("DOT", "DOT"),
        ("DOGECOIN", "DOGE"), ("DOGE", "DOGE"), ("BINANCE COIN", "BNB"), ("BNB", "BNB"),
        ("SOLANA", "SOL"), ("SOL", "SOL"), ("POLYGON", "MATIC"), ("MATIC", "MATIC"),
        ("AVALANCHE", "AVAX"), ("AVAX", "AVAX"), ("CHAINLINK", "LINK"), ("LINK", "LINK"),
        ("UNISWAP", "UNI"), ("UNI", "UNI"), ("LITECOIN", "LTC"), ("LTC", "LTC"),
        ("HEDERA", "HBAR"), ("HBAR", "HBAR"), ("BTC", "BTC"), ("ETH", "ETH"),
        ("SHIBA INU", "SHIB"), ("SHIB", "SHIB"), ("TRON", "TRX"), ("TRX", "TRX"),
        ("STELLAR", "XLM"), ("XLM", "XLM"), ("COSMOS", "ATOM"), ("ATOM", "ATOM")
    ];

    let mut found_assets = Vec::new();
    
    for (name, symbol) in crypto_symbols {
        if text_upper.contains(name) {
            // Try to find percentage
            if let Some(percentage) = extract_percentage_for_symbol(&text_upper, symbol) {
                found_assets.push(format!("{} ({})", symbol, percentage));
            } else {
                found_assets.push(symbol.to_string());
            }
            
            // Only add up to 2 assets per article to avoid clutter
            if found_assets.len() >= 2 {
                break;
            }
        }
    }

    found_assets
}

fn extract_percentage_for_symbol(text: &str, symbol: &str) -> Option<String> {
    // Look for patterns like "BTC +2.5%" or "ETH (-1.2%)" or "up 5%" near the symbol
    let patterns = vec![
        format!(r"{}\s*[\+\-]?\s*(\d+\.?\d*%)", symbol),
        format!(r"(\+\d+\.?\d*%|\-\d+\.?\d*%)\s*{}", symbol),
        format!(r"{}\s+(?:up|down|gained?|lost?)\s+(\d+\.?\d*%)", symbol),
        format!(r"(?:up|down|gained?|lost?)\s+(\d+\.?\d*%)\s*{}", symbol),
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(text) {
                if let Some(percentage) = captures.get(1) {
                    return Some(percentage.as_str().to_string());
                }
            }
        }
    }
    None
}

fn clean_html(text: &str) -> String {
    // Remove HTML tags and decode entities
    let tag_pattern = regex::Regex::new(r"<[^>]*>").unwrap();
    let cleaned = tag_pattern.replace_all(text, "");
    
    // Decode common HTML entities
    cleaned
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .trim()
        .to_string()
}
