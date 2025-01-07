use std::error::Error;

use serde_json::Value;

/// Represents the base trait for all tools
#[allow(dead_code)]
pub trait BaseTool {
    /// Tool name that matches the regex ^[a-zA-Z0-9_-]{1,64}$
    fn name(&self) -> &str;

    /// Detailed description of what the tool does
    fn description(&self) -> &str;

    /// JSON Schema defining the expected parameters
    fn input_schema(&self) -> &Value;

    /// Execute the tool with given parameters
    fn execute(&self, params: Value) -> Result<String, Box<dyn Error>>;
}
