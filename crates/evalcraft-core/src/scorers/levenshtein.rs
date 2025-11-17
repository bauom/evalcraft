use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use strsim::levenshtein;

use crate::scorer::Scorer;
use crate::types::Score;

pub struct LevenshteinScorer {
    pub min_similarity: f64,
}

impl LevenshteinScorer {
    pub fn new(min_similarity: f64) -> Self {
        Self { min_similarity }
    }
}

#[async_trait]
impl Scorer for LevenshteinScorer {
    fn name(&self) -> &'static str {
        "levenshtein"
    }

    async fn score(&self, expected: &Value, output: &Value) -> Result<Score> {
        let e = stringify(expected)?;
        let o = stringify(output)?;
        let max_len = e.len().max(o.len()).max(1) as f64;
        let similarity = 1.0 - (levenshtein(&e, &o) as f64 / max_len);
        let passed = similarity >= self.min_similarity;
        Ok(Score {
            name: self.name().to_string(),
            value: similarity,
            passed,
            details: None,
        })
    }
}

fn stringify(v: &Value) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Null => Ok(String::new()),
        _ => Ok(v.to_string()),
    }
}
