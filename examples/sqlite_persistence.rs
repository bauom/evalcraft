use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;
use evalcraft_store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Example: SQLite Persistence ===\n");
    
    // 1. Run the evaluation (same as tracing_example)
    let result = run_eval().await?;
    println!("{}", result.summary_table());

    // 2. Persist to SQLite
    let db_path = "eval_history.db";
    
    println!("Opening SQLite store at '{}'...", db_path);
    let store = Store::open(db_path)?;

    // Create a Run
    let run_id = store.create_run(Some(json!({
        "environment": "local",
        "user": "example_runner"
    })))?;
    println!("Created Run ID: {}", run_id);

    // Save the Eval Result associated with this Run
    let eval_id = store.save_eval(run_id, "Capital Cities Eval", &result)?;
    println!("Saved Eval ID: {}", eval_id);

    println!("\n✓ Data successfully persisted to SQLite.");
    println!("  You can inspect it with: sqlite3 {} 'select * from results;'", db_path);

    Ok(())
}

async fn run_eval() -> anyhow::Result<EvalResult> {
    // Simulated LLM function
    async fn mock_llm_call(prompt: &str) -> anyhow::Result<String> {
        let trace_builder = Trace::start_now()
            .model("gpt-4o-mini")
            .id("call-1");
        
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let response = if prompt.contains("France") {
            "Paris"
        } else {
            "Unknown"
        };
        
        let trace = trace_builder.finish(
            json!({"prompt": prompt}),
            json!({"response": response}),
            Some(TokenUsage {
                input_tokens: 15,
                output_tokens: 5,
                total_tokens: 20,
            }),
        );
        report_trace(trace);
        
        Ok(response.to_string())
    }
    
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let response = mock_llm_call(prompt).await?;
            Ok(json!(response))
        }
    });
    
    let cases = vec![
        TestCase::with_id("case-1", json!("Capital of France?"), json!("Paris")),
        TestCase::with_id("case-2", json!("Capital of Germany?"), json!("Berlin")),
    ];
    let data = Arc::new(VecDataSource::new(cases));
    
    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("Paris")), // Only checks strict containment
    ];
    
    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .build()?;
    
    eval.run().await
}

