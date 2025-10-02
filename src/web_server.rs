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

pub async fn create_router() -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/logs", get(list_log_files))
        .route("/api/logs/{filename}", get(get_log_entries))
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