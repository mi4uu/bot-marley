use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, get_service},
    Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tracing::{error, info};
use color_eyre::eyre::{Result, WrapErr};
use regex::Regex;

use crate::portfolio::{load_portfolio_history, get_portfolio_summary, PortfolioSnapshot, PortfolioSummary};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub timestamp_utc: Option<String>,
    pub level: String,
    pub target: String,
    pub module: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: serde_json::Value,
    pub symbol: Option<String>,
    pub run: Option<u32>,
    pub turn: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub level: Option<String>,
    pub symbol: Option<String>,
    pub run: Option<u32>,
    pub turn: Option<u32>,
    pub target: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct LogFile {
    pub filename: String,
    pub date: String,
    pub hour: String,
    pub size: u64,
    pub entry_count: usize,
}

#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub entries: Vec<LogEntry>,
    pub total: usize,
    pub filtered: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct MarkdownFile {
    pub filename: String,
    pub title: String,
    pub date: Option<String>,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Serialize)]
pub struct MarkdownContent {
    pub filename: String,
    pub title: String,
    pub content: String,
    pub toc: Vec<TocEntry>,
}

#[derive(Debug, Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub anchor: String,
}

pub async fn create_router() -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/logs", get(list_log_files))
        .route("/api/logs/{filename}", get(get_log_entries))
        .route("/api/markdown", get(list_markdown_files))
        .route("/api/markdown/{filename}", get(get_markdown_content))
        .route("/api/portfolio", get(get_portfolio_data))
        .route("/api/portfolio/summary", get(get_portfolio_summary_endpoint))
        .nest_service("/static", get_service(ServeDir::new("static")))
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn list_log_files() -> Result<Json<Vec<LogFile>>, StatusCode> {
    let logs_dir = PathBuf::from("logs");
    
    if !logs_dir.exists() {
        return Ok(Json(vec![]));
    }

    let mut log_files = Vec::new();
    
    match fs::read_dir(&logs_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            if filename.starts_with("botmarley.") {
                                if let Ok(metadata) = entry.metadata() {
                                    let entry_count = count_log_entries(&path).unwrap_or_else(|e| {
                                        error!("Failed to count entries in {:?}: {:?}", path, e);
                                        0
                                    });
                                    
                                    // Parse date and hour from filename
                                    let parts: Vec<&str> = filename.split('.').collect();
                                    if parts.len() >= 2 {
                                        
                                        let date_hour = parts[1];
                                        let date_parts: Vec<&str> = date_hour.split('-').collect();
                                        if date_parts.len() == 4 {
                                            let date = format!("{}-{}-{}", date_parts[0], date_parts[1], date_parts[2]);
                                            let hour = date_parts[3].to_string();
                                            
                                            log_files.push(LogFile {
                                                filename: filename.to_string(),
                                                date,
                                                hour,
                                                size: metadata.len(),
                                                entry_count,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to read logs directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Sort by filename (which includes timestamp)
    log_files.sort_by(|a, b| b.filename.cmp(&a.filename));
    
    Ok(Json(log_files))
}

async fn get_log_entries(
    Path(filename): Path<String>,
    Query(filter): Query<LogFilter>,
) -> Result<Json<LogResponse>, StatusCode> {
    info!("get_log_entries called with filename: {}, filter: {:?}", filename, filter);
    let log_path = PathBuf::from("logs").join(&filename);
    
    if !log_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match fs::read_to_string(&log_path) {
        Ok(content) => {
            let mut entries = Vec::new();
            let mut total = 0;
            
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                
                total += 1;
                
                match serde_json::from_str::<LogEntry>(line) {
                    Ok(entry) => {
                        // Apply filters
                        if let Some(ref level_filter) = filter.level {
                            if entry.level.to_lowercase() != level_filter.to_lowercase() {
                                continue;
                            }
                        }
                        
                        if let Some(ref symbol_filter) = filter.symbol {
                            if entry.symbol.as_ref().map(|s| s.as_str()) != Some(symbol_filter) {
                                continue;
                            }
                        }
                        
                        if let Some(run_filter) = filter.run {
                            if entry.run != Some(run_filter) {
                                continue;
                            }
                        }
                        
                        if let Some(turn_filter) = filter.turn {
                            if entry.turn != Some(turn_filter) {
                                continue;
                            }
                        }
                        
                        if let Some(ref target_filter) = filter.target {
                            if !entry.target.contains(target_filter) {
                                continue;
                            }
                        }
                        
                        entries.push(entry);
                    }
                    Err(e) => {
                        error!("Failed to parse log entry: {} - Line: {}", e, line);
                        // Try to parse as old format
                        if let Ok(old_entry) = parse_old_format_log(line) {
                            entries.push(old_entry);
                        }
                    }
                }
            }
            
            let filtered_count = entries.len();
            let offset = filter.offset.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            
            // Apply pagination
            let has_more = offset + limit < filtered_count;
            entries = entries.into_iter()
                .skip(offset)
                .take(limit)
                .collect();
            
            Ok(Json(LogResponse {
                entries,
                total,
                filtered: filtered_count,
                has_more,
            }))
        }
        Err(e) => {
            error!("Failed to read log file {}: {}", filename, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn count_log_entries(path: &PathBuf) -> Result<usize> {
    let content = fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read log file: {:?}", path))?;
    Ok(content.lines().filter(|line| !line.trim().is_empty()).count())
}

fn parse_old_format_log(line: &str) -> Result<LogEntry, serde_json::Error> {
    // Try to parse old JSON format
    let value: serde_json::Value = serde_json::from_str(line)?;
    
    // Convert old format to new format
    let entry = LogEntry {
        timestamp: value["timestamp"].as_str().unwrap_or("").to_string(),
        timestamp_utc: Some(value["timestamp"].as_str().unwrap_or("").to_string()),
        level: value["level"].as_str().unwrap_or("INFO").to_string(),
        target: value["target"].as_str().unwrap_or("").to_string(),
        module: None,
        file: None,
        line: None,
        fields: value["fields"].clone(),
        symbol: None,
        run: None,
        turn: None,
    };
    
    Ok(entry)
}

async fn list_markdown_files() -> Result<Json<Vec<MarkdownFile>>, StatusCode> {
    let logs_md_dir = PathBuf::from("logs_md");
    
    if !logs_md_dir.exists() {
        return Ok(Json(vec![]));
    }

    let mut markdown_files = Vec::new();
    
    match fs::read_dir(&logs_md_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            if filename.ends_with(".md") {
                                if let Ok(metadata) = entry.metadata() {
                                    // Extract title from filename or first line of file
                                    let title = extract_title_from_filename(filename);
                                    let date = extract_date_from_filename(filename);
                                    let modified = format!("{:?}", metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));
                                    
                                    markdown_files.push(MarkdownFile {
                                        filename: filename.to_string(),
                                        title,
                                        date,
                                        size: metadata.len(),
                                        modified,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to read logs_md directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Sort by filename (which includes timestamp)
    markdown_files.sort_by(|a, b| b.filename.cmp(&a.filename));
    
    Ok(Json(markdown_files))
}

async fn get_markdown_content(
    Path(filename): Path<String>,
) -> Result<Json<MarkdownContent>, StatusCode> {
    let markdown_path = PathBuf::from("logs_md").join(&filename);
    
    if !markdown_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match fs::read_to_string(&markdown_path) {
        Ok(content) => {
            let title = extract_title_from_content(&content).unwrap_or_else(|| extract_title_from_filename(&filename));
            let toc = generate_toc(&content);
            let processed_content = process_markdown_content(&content);
            
            Ok(Json(MarkdownContent {
                filename: filename.clone(),
                title,
                content: processed_content,
                toc,
            }))
        }
        Err(e) => {
            error!("Failed to read markdown file {}: {}", filename, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn extract_title_from_filename(filename: &str) -> String {
    // Remove .md extension and convert underscores/hyphens to spaces
    filename
        .strip_suffix(".md")
        .unwrap_or(filename)
        .replace('_', " ")
        .replace('-', " ")
}

fn extract_date_from_filename(filename: &str) -> Option<String> {
    // Try to extract date from filename like "2025-10-02_19-01.md"
    let re = Regex::new(r"(\d{4}-\d{2}-\d{2})").ok()?;
    re.captures(filename)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_title_from_content(content: &str) -> Option<String> {
    // Look for the first # header
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].trim().to_string());
        }
    }
    None
}

fn generate_toc(content: &str) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    let header_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    
    for line in content.lines() {
        if let Some(captures) = header_re.captures(line.trim()) {
            let level = captures.get(1).unwrap().as_str().len() as u8;
            let title = captures.get(2).unwrap().as_str().to_string();
            let anchor = title
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect();
            
            toc.push(TocEntry {
                level,
                title,
                anchor,
            });
        }
    }
    
    toc
}

fn process_markdown_content(content: &str) -> String {
    // Add anchors to headers and make <think></think> sections collapsible
    let header_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    let think_start_re = Regex::new(r"<think>").unwrap();
    let think_end_re = Regex::new(r"</think>").unwrap();
    
    let mut processed_lines = Vec::new();
    let mut in_think_block = false;
    
    for line in content.lines() {
        let mut processed_line = line.to_string();
        
        // Process headers to add anchors
        if let Some(captures) = header_re.captures(line.trim()) {
            let hashes = captures.get(1).unwrap().as_str();
            let title = captures.get(2).unwrap().as_str();
            let anchor = title
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            
            processed_line = format!("{} <a id=\"{}\"></a>{}", hashes, anchor, title);
        }
        
        // Process <think> tags
        if think_start_re.is_match(&processed_line) {
            processed_line = think_start_re.replace(&processed_line, "<details class=\"think-block\"><summary>💭 Thinking...</summary><div class=\"think-content\">").to_string();
            in_think_block = true;
        } else if think_end_re.is_match(&processed_line) {
            processed_line = think_end_re.replace(&processed_line, "</div></details>").to_string();
            in_think_block = false;
        }
        
        processed_lines.push(processed_line);
    }
    
    processed_lines.join("\n")
}

async fn get_portfolio_data() -> Result<Json<Vec<PortfolioSnapshot>>, StatusCode> {
    match load_portfolio_history() {
        Ok(snapshots) => Ok(Json(snapshots)),
        Err(e) => {
            error!("Failed to load portfolio history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_portfolio_summary_endpoint() -> Result<Json<Option<PortfolioSummary>>, StatusCode> {
    match load_portfolio_history() {
        Ok(snapshots) => {
            let summary = get_portfolio_summary(&snapshots);
            Ok(Json(summary))
        }
        Err(e) => {
            error!("Failed to load portfolio history for summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn start_web_server(port: u16) -> Result<()> {
    let app = create_router().await;
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .wrap_err_with(|| format!("Failed to bind to port {}", port))?;
    info!("🌐 Web server starting on http://localhost:{}", port);
    info!("📊 Log viewer available at http://localhost:{}", port);
    
    axum::serve(listener, app).await
        .wrap_err("Failed to start web server")?;
    
    Ok(())
}