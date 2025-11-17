use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

/// Checks if output matches a regex pattern.
pub struct RegexScorer {
	pattern: Regex,
	pattern_str: String,
}

impl RegexScorer {
	/// Creates a regex scorer with the given pattern.
	pub fn new(pattern: &str) -> Result<Self> {
		let regex = Regex::new(pattern)?;
		Ok(Self {
			pattern: regex,
			pattern_str: pattern.to_string(),
		})
	}
}

#[async_trait]
impl Scorer for RegexScorer {
	fn name(&self) -> &'static str {
		"regex"
	}

	async fn score(&self, _expected: &Value, output: &Value) -> Result<Score> {
		let output_str = match output {
			Value::String(s) => s.clone(),
			_ => serde_json::to_string(output)?,
		};

		let matches = self.pattern.is_match(&output_str);

		Ok(Score {
			name: self.name().to_string(),
			value: if matches { 1.0 } else { 0.0 },
			passed: matches,
			details: Some(serde_json::json!({
				"pattern": self.pattern_str,
				"matches": matches,
				"captures": if matches {
					self.pattern.captures(&output_str)
						.map(|caps| {
							caps.iter()
								.enumerate()
								.filter_map(|(i, m)| m.map(|m| (i, m.as_str().to_string())))
								.collect::<Vec<_>>()
						})
				} else {
					None
				}
			})),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_regex_match() {
		let scorer = RegexScorer::new(r"capital.*Paris").unwrap();
		let output = serde_json::json!("The capital of France is Paris");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).await.unwrap();
		assert!(score.passed);
		assert_eq!(score.value, 1.0);
	}

	#[tokio::test]
	async fn test_regex_no_match() {
		let scorer = RegexScorer::new(r"capital.*London").unwrap();
		let output = serde_json::json!("The capital of France is Paris");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).await.unwrap();
		assert!(!score.passed);
		assert_eq!(score.value, 0.0);
	}

	#[tokio::test]
	async fn test_regex_email() {
		let scorer = RegexScorer::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
		let output = serde_json::json!("user@example.com");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).await.unwrap();
		assert!(score.passed);
		assert_eq!(score.value, 1.0);
	}

	#[tokio::test]
	async fn test_regex_with_capture_groups() {
		let scorer = RegexScorer::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
		let output = serde_json::json!("Date: 2024-11-12");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).await.unwrap();
		assert!(score.passed);
		assert_eq!(score.value, 1.0);
	}
}

