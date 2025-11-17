use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

/// Checks if output contains a substring.
pub struct ContainsScorer {
    substring: String,
    case_sensitive: bool,
}

impl ContainsScorer {
    /// Creates a case-sensitive contains scorer.
    pub fn new(substring: impl Into<String>) -> Self {
        Self {
            substring: substring.into(),
            case_sensitive: true,
        }
    }

    /// Creates a case-insensitive contains scorer.
    pub fn case_insensitive(substring: impl Into<String>) -> Self {
        Self {
            substring: substring.into(),
            case_sensitive: false,
        }
    }
}

#[async_trait]
impl Scorer for ContainsScorer {
    fn name(&self) -> &'static str {
        "contains"
    }

    async fn score(&self, _expected: &Value, output: &Value) -> Result<Score> {
        let output_str = match output {
            Value::String(s) => s.clone(),
            _ => serde_json::to_string(output)?,
        };

        let contains = if self.case_sensitive {
            output_str.contains(&self.substring)
        } else {
            output_str
                .to_lowercase()
                .contains(&self.substring.to_lowercase())
        };

        Ok(Score {
            name: self.name().to_string(),
            value: if contains { 1.0 } else { 0.0 },
            passed: contains,
            details: Some(serde_json::json!({
                "substring": self.substring,
                "case_sensitive": self.case_sensitive,
                "found": contains
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_contains_found() {
        let scorer = ContainsScorer::new("Paris");
        let output = serde_json::json!("The capital of France is Paris");
        let expected = serde_json::json!("");
        let score = scorer.score(&expected, &output).await.unwrap();
        assert!(score.passed);
        assert_eq!(score.value, 1.0);
    }

    #[tokio::test]
    async fn test_contains_not_found() {
        let scorer = ContainsScorer::new("London");
        let output = serde_json::json!("The capital of France is Paris");
        let expected = serde_json::json!("");
        let score = scorer.score(&expected, &output).await.unwrap();
        assert!(!score.passed);
        assert_eq!(score.value, 0.0);
    }

    #[tokio::test]
    async fn test_contains_case_insensitive() {
        let scorer = ContainsScorer::case_insensitive("PARIS");
        let output = serde_json::json!("The capital of France is paris");
        let expected = serde_json::json!("");
        let score = scorer.score(&expected, &output).await.unwrap();
        assert!(score.passed);
        assert_eq!(score.value, 1.0);
    }

    #[tokio::test]
    async fn test_contains_case_sensitive_fail() {
        let scorer = ContainsScorer::new("PARIS");
        let output = serde_json::json!("The capital of France is Paris");
        let expected = serde_json::json!("");
        let score = scorer.score(&expected, &output).await.unwrap();
        assert!(!score.passed);
        assert_eq!(score.value, 0.0);
    }
}
