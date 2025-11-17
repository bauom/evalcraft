use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use evalcraft_core::*;
use reqwest::Client;
use serde_json::json;

type EmbedFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Localhost LLM ===\n");

    run_localhost_llm().await?;

    Ok(())
}

fn make_embed_fn() -> Arc<dyn for<'a> Fn(&'a str) -> EmbedFuture<'a> + Send + Sync> {
    let client = Client::new();
    let endpoint = "http://localhost:8080/embeddings".to_string();

    Arc::new(move |text: &str| {
        let client = client.clone();
        let endpoint = endpoint.clone();
        let input = text.to_owned();

        Box::pin(async move {
            let resp = client
                .post(&endpoint)
                .json(&json!({ "input": input }))
                .send()
                .await?;
            let v: serde_json::Value = resp.json().await?;

            // Support shape: [{"index":0,"embedding":[[...]]}]
            let arr = v
                .as_array()
                .and_then(|outer| outer.get(0))
                .and_then(|obj| obj.get("embedding"))
                .and_then(|emb| emb.as_array())
                .and_then(|emb_outer| emb_outer.get(0))
                .and_then(|inner| inner.as_array())
                .ok_or_else(|| anyhow::anyhow!("unexpected embedding response shape"))?;

            let embedding = arr
                .iter()
                .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                .collect();

            Ok(embedding)
        })
    })
}

async fn run_localhost_llm() -> anyhow::Result<()> {
    let task = from_async_fn(|input| {
        let input = input.clone();
        let model = "Qwen3-0.5b";
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let system_prompt =
                "You are a helpful assistant. try to reply to answers as concise as possible, 1 word is enough.";
            let response = call_OAI_Compat(system_prompt, prompt, model).await?;
            Ok(json!(response))
        }
    });

    let data = Arc::new(JsonlDataSource::new("examples/openai_tests.jsonl"));

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(LevenshteinScorer::new(0.6)),
        Arc::new(EmbeddingScorer::new(make_embed_fn(), 0.6)),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(1)
        .build();

    let result = eval.run().await?;

    // Existing summary table (one row per case with avg_score)
    println!("{}", result.summary_table());

    // Additional detailed scores for each scorer per case
    println!("\nDetailed scores per case:");
    for cr in &result.cases {
        let id = cr.case.id.as_deref().unwrap_or("-");
        println!("Case: {}", id);
        for s in &cr.scores {
            println!("  {}: value={:.3}, passed={}", s.name, s.value, s.passed);
        }
        if let Some(err) = &cr.error {
            println!("  error: {}", err);
        }
        println!();
    }

    let json_result = serde_json::to_string_pretty(&result);
    let datetime: DateTime<Utc> = Utc::now();
    tokio::fs::write(
        &format!(
            "localhost_llm_results_{}.json",
            datetime.format("%Y-%m-%d_%H-%M-%S")
        ),
        json_result.unwrap(),
    )
    .await?;
    println!("Results saved to localhost_llm_results_{}.json", datetime);

    Ok(())
}

async fn call_OAI_Compat(system_prompt: &str, prompt: &str, model: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let response = client
        .post("http://localhost:8080/chat/completions")
        .json(&json!({
            "model": model,
            "messages": [{"role": "system", "content": system_prompt}, {"role": "user", "content": prompt}],
            "temperature": 0.0
        }))
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(content)
}
