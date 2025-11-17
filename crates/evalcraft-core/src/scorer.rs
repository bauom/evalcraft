use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::types::Score;

#[async_trait]
pub trait Scorer: Send + Sync {
    fn name(&self) -> &'static str;
    async fn score(&self, expected: &Value, output: &Value) -> Result<Score>;
}
