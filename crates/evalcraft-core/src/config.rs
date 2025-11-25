use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    pub task: TaskConfig,
    pub data: DataConfig,
    #[serde(default)]
    pub scorers: Vec<ScorerConfig>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_concurrency() -> usize {
    8
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum TaskConfig {
    Http {
        url: String,
        #[serde(default = "default_http_method")]
        method: String,
    },
}

fn default_http_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ScorerConfig {
    Exact,
    Levenshtein {
        threshold: f64,
    },
    Contains {
        substring: String,
        #[serde(default)]
        case_sensitive: bool,
    },
    Regex {
        pattern: String,
    },
    Json,
    JsonSchema {
        path: PathBuf,
    },
    Sql {
        #[serde(default)]
        dialect: String,
    },
}

