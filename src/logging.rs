use std::io;
use chrono::{Local, Utc, Timelike};
use serde_json::json;
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, FormatEvent, FormatFields},
    registry::LookupSpan,
};
use std::fmt;
use color_eyre::eyre::{Result, WrapErr};

pub struct CustomJsonFormatter;

impl<S, N> FormatEvent<S, N> for CustomJsonFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let now = Local::now();
        
        // Get current symbol from span context if available
        let mut symbol = None;
        let mut run_number = None;
        let mut turn_number = None;
        
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<tracing_subscriber::fmt::FormattedFields<N>>() {
                    let fields_str = fields.as_str();
                    // Parse symbol from span fields
                    if fields_str.contains("symbol") {
                        // Extract symbol value from formatted fields
                        if let Some(start) = fields_str.find("symbol: ") {
                            let start = start + 8;
                            if let Some(end) = fields_str[start..].find(',').or_else(|| fields_str[start..].find('}')) {
                                symbol = Some(fields_str[start..start + end].trim_matches('"').to_string());
                            }
                        }
                    }
                    if fields_str.contains("run") {
                        if let Some(start) = fields_str.find("run: ") {
                            let start = start + 5;
                            if let Some(end) = fields_str[start..].find(',').or_else(|| fields_str[start..].find('}')) {
                                if let Ok(run) = fields_str[start..start + end].parse::<u32>() {
                                    run_number = Some(run);
                                }
                            }
                        }
                    }
                    if fields_str.contains("turn") {
                        if let Some(start) = fields_str.find("turn: ") {
                            let start = start + 6;
                            if let Some(end) = fields_str[start..].find(',').or_else(|| fields_str[start..].find('}')) {
                                if let Ok(turn) = fields_str[start..start + end].parse::<u32>() {
                                    turn_number = Some(turn);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Collect event fields
        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);

        let mut log_entry = json!({
            "timestamp": now.to_rfc3339(),
            "timestamp_utc": Utc::now().to_rfc3339(),
            "level": metadata.level().to_string(),
            "target": metadata.target(),
            "module": metadata.module_path(),
            "file": metadata.file(),
            "line": metadata.line(),
            "fields": visitor.fields
        });

        // Add metadata if available
        if let Some(sym) = symbol {
            log_entry["symbol"] = json!(sym);
        }
        if let Some(run) = run_number {
            log_entry["run"] = json!(run);
        }
        if let Some(turn) = turn_number {
            log_entry["turn"] = json!(turn);
        }

        writeln!(writer, "{}", log_entry)?;
        Ok(())
    }
}

struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
        }
    }
}

impl tracing::field::Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            json!(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }
}

// Custom file appender that uses local time for filenames
pub struct LocalTimeFileAppender {
    directory: std::path::PathBuf,
    file_name_prefix: String,
    current_file: Option<std::fs::File>,
    current_hour: u32,
}

impl LocalTimeFileAppender {
    pub fn new<P: AsRef<std::path::Path>>(directory: P, file_name_prefix: &str) -> Result<Self> {
        let directory = directory.as_ref().to_path_buf();
        std::fs::create_dir_all(&directory)
            .wrap_err("Failed to create logs directory")?;
        
        Ok(Self {
            directory,
            file_name_prefix: file_name_prefix.to_string(),
            current_file: None,
            current_hour: 25, // Invalid hour to force initial file creation
        })
    }

    fn get_current_filename(&self) -> String {
        let now = Local::now();
        format!(
            "{}.{}.log",
            self.file_name_prefix,
            now.format("%Y-%m-%d-%H")
        )
    }

    fn ensure_current_file(&mut self) -> Result<&mut std::fs::File> {
        let now = Local::now();
        let current_hour = now.hour();

        if self.current_hour != current_hour || self.current_file.is_none() {
            let filename = self.get_current_filename();
            let filepath = self.directory.join(filename);
            
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filepath)
                .wrap_err_with(|| format!("Failed to open log file: {:?}", filepath))?;
            
            self.current_file = Some(file);
            self.current_hour = current_hour;
        }

        Ok(self.current_file.as_mut().unwrap())
    }
}

impl io::Write for LocalTimeFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let file = self.ensure_current_file()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = &mut self.current_file {
            file.flush()
        } else {
            Ok(())
        }
    }
}