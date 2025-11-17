use std::sync::Arc;

use evalcraft_core::{
    from_async_fn, Eval, ExactMatchScorer, JsonlDataSource, LevenshteinScorer, Scorer, TestCase,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Example 1: Inline cases
    let cases = vec![
        TestCase::with_id("0", json!("Hello"), json!("Hello World!")),
        TestCase::with_id("1", json!("Hi"), json!("Hi World!")),
    ];
    let data = Arc::new(evalcraft_core::VecDataSource::new(cases));

    // Task: append " World!" to any string input
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(format!("{s} World!")))
        }
    });

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ExactMatchScorer),
        Arc::new(LevenshteinScorer::new(0.9)),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(8)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());

    // Example 2: Load from JSONL file if provided
    if let Some(path) = std::env::args().nth(1) {
        let data = Arc::new(JsonlDataSource::new(path));
        let scorers: Vec<Arc<dyn Scorer>> = vec![Arc::new(ExactMatchScorer)];
        let eval = Eval::builder()
            .data_source(data)
            .task(from_async_fn(|input| {
                let input = input.clone();
                async move {
                    let s = input.as_str().unwrap_or_default();
                    Ok(json!(format!("{s} World!")))
                }
            }))
            .scorers(scorers)
            .build();
        let result = eval.run().await?;
        println!("{}", result.summary_table());
    }

    Ok(())
}
