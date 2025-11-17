use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

type EmbedFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;

pub struct EmbeddingScorer {
    embed_fn: Arc<dyn for<'a> Fn(&'a str) -> EmbedFuture<'a> + Send + Sync>,
    pub min_similarity: f64,
}

impl EmbeddingScorer {
    pub fn new(
        embed_fn: Arc<dyn for<'a> Fn(&'a str) -> EmbedFuture<'a> + Send + Sync>,
        min_similarity: f64,
    ) -> Self {
        Self {
            embed_fn,
            min_similarity,
        }
    }
}

#[async_trait]
impl Scorer for EmbeddingScorer {
    fn name(&self) -> &'static str {
        "embedding_cosine"
    }

    async fn score(&self, expected: &Value, output: &Value) -> Result<Score> {
        let expected_str = match expected {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => serde_json::to_string(expected)?,
        };

        let output_str = match output {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => serde_json::to_string(output)?,
        };

        let e_vec = (self.embed_fn)(&expected_str).await?;
        let o_vec = (self.embed_fn)(&output_str).await?;

        let similarity = cosine_similarity(&e_vec, &o_vec);

        let passed = similarity >= self.min_similarity;

        Ok(Score {
            name: self.name().to_string(),
            value: similarity,
            passed,
            details: None,
        })
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (a_val, b_val) in a.iter().zip(b.iter()) {
        let x = *a_val as f64;
        let y = *b_val as f64;

        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}
