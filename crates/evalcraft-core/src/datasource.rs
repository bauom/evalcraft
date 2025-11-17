use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::types::TestCase;

#[async_trait]
pub trait DataSource: Send + Sync {
    async fn load(&self) -> Result<Vec<TestCase>>;
}

pub struct VecDataSource {
    cases: Vec<TestCase>,
}

impl VecDataSource {
    pub fn new(cases: Vec<TestCase>) -> Self {
        Self { cases }
    }
}

#[async_trait]
impl DataSource for VecDataSource {
    async fn load(&self) -> Result<Vec<TestCase>> {
        Ok(self.cases.clone())
    }
}

/// Read JSONL where each line is either:
/// - {"id": "...", "input": ..., "expected": ...}
/// - {"input": ..., "expected": ...}
pub struct JsonlDataSource {
    path: PathBuf,
}

impl JsonlDataSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl DataSource for JsonlDataSource {
    async fn load(&self) -> Result<Vec<TestCase>> {
        let content = tokio_fs_read_to_string(&self.path).await?;
        let mut cases = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(line)
                .with_context(|| format!("Invalid JSON on line {}", idx + 1))?;
            let obj = value
                .as_object()
                .ok_or_else(|| anyhow!("Line {}: expected object", idx + 1))?;
            let input = obj
                .get("input")
                .cloned()
                .ok_or_else(|| anyhow!("Line {}: missing 'input'", idx + 1))?;
            let expected = obj
                .get("expected")
                .cloned()
                .ok_or_else(|| anyhow!("Line {}: missing 'expected'", idx + 1))?;
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cases.push(TestCase {
                id,
                input,
                expected,
            });
        }
        Ok(cases)
    }
}

#[cfg(not(feature = "sync-fs"))]
async fn tokio_fs_read_to_string(path: &PathBuf) -> Result<String> {
    use tokio::fs;
    Ok(fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read {:?}", path))?)
}

#[cfg(feature = "sync-fs")]
async fn tokio_fs_read_to_string(path: &PathBuf) -> Result<String> {
    use std::fs;
    use tokio::task;
    let path_clone = path.clone();
    let content = task::spawn_blocking(move || {
        fs::read_to_string(&path_clone).with_context(|| format!("Failed to read {:?}", path_clone))
    })
    .await
    .map_err(|e| anyhow!(e))??;
    Ok(content)
}
