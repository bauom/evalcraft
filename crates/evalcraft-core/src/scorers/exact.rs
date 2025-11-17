use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

pub struct ExactMatchScorer;

#[async_trait]
impl Scorer for ExactMatchScorer {
	fn name(&self) -> &'static str {
		"exact_match"
	}

	async fn score(&self, expected: &Value, output: &Value) -> Result<Score> {
		let passed = expected == output;
		let value = if passed { 1.0 } else { 0.0 };
		Ok(Score {
			name: self.name().to_string(),
			value,
			passed,
			details: None,
		})
	}
}


