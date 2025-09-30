use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub function: Arc<dyn Fn(serde_json::Value) -> String + Send + Sync>,
}