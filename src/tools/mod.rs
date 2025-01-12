mod base;

use lazy_static::lazy_static;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use walkdir::WalkDir;

use base::BaseTool;

lazy_static! {
    static ref LIST_FILES_SCHEMA: Value = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string"
            }
        },
        "required": ["path"]
    });
    static ref SEARCH_FILES_SCHEMA: Value = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string"
            },
            "pattern": {
                "type": "string"
            }
        },
        "required": ["path", "pattern"]
    });
    static ref FILE_INFO_SCHEMA: Value = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string"
            }
        },
        "required": ["path"]
    });
}

/// A tool for listing files in a directory
pub struct ListFiles;

/// A tool for searching files matching a pattern
pub struct SearchFiles;

/// A tool for getting file metadata
pub struct FileInfo;

impl BaseTool for ListFiles {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "Lists files within a directory. Only works within allowed directories."
    }

    fn input_schema(&self) -> &Value {
        &LIST_FILES_SCHEMA
    }

    fn execute(&self, params: Value) -> Result<String, Box<dyn Error>> {
        let path = params["path"].as_str().ok_or("Path parameter required")?;
        let entries = fs::read_dir(path)?;
        let files: Vec<String> = entries
            .flatten()
            .map(|entry| entry.path().display().to_string())
            .collect();

        Ok(serde_json::to_string_pretty(&files)?)
    }
}

impl BaseTool for SearchFiles {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Recursively search for files and directories matching a pattern. \
        Searches through all subdirectories from the starting path."
    }

    fn input_schema(&self) -> &Value {
        &SEARCH_FILES_SCHEMA
    }

    fn execute(&self, params: Value) -> Result<String, Box<dyn Error>> {
        let path = params["path"].as_str().ok_or("Path parameter required")?;
        let pattern = params["pattern"]
            .as_str()
            .ok_or("Pattern parameter required")?;
        let pattern = pattern.to_lowercase();

        let matches: Vec<String> = WalkDir::new(path)
            .into_iter()
            .flatten()
            .map(|entry| entry.path().display().to_string())
            .filter(|path_str| path_str.to_lowercase().contains(&pattern))
            .collect();

        Ok(serde_json::to_string_pretty(&matches)?)
    }
}

impl BaseTool for FileInfo {
    fn name(&self) -> &str {
        "get_file_info"
    }

    fn description(&self) -> &str {
        "Retrieve detailed metadata about a file or directory."
    }

    fn input_schema(&self) -> &Value {
        &FILE_INFO_SCHEMA
    }

    fn execute(&self, params: Value) -> Result<String, Box<dyn Error>> {
        let path = params["path"].as_str().ok_or("Path parameter required")?;
        let path = Path::new(path);
        let metadata = fs::metadata(path)?;

        let info = json!({
            "path": path.display().to_string(),
            "size": metadata.len(),
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
            "created": metadata.created()?.duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            "modified": metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            "permissions": format!("{:o}", metadata.permissions().mode()),
        });

        Ok(serde_json::to_string_pretty(&info)?)
    }
}
