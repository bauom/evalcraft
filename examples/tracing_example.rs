// Example: Using Traces to Debug LLM Calls
//
// This shows how to use the tracing system to capture detailed information
// about LLM calls, including timing, token usage, and inputs/outputs.
//
// To run from the workspace root:
//   cargo run -p evalcraft-core --example tracing_example

use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Example: Tracing LLM Calls ===\n");
    
    run_traced_example().await?;
    
    Ok(())
}

async fn run_traced_example() -> anyhow::Result<()> {
    // Simulated LLM function that reports traces
    async fn mock_llm_call(prompt: &str) -> anyhow::Result<String> {
        // Start timing
        let trace_builder = Trace::start_now()
            .model("gpt-4o-mini")
            .id("call-1");
        
        // Simulate LLM call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let response = if prompt.contains("France") {
            "The capital of France is Paris."
        } else {
            "I don't know."
        };
        
        // Report trace with usage info
        let trace = trace_builder.finish(
            json!({
                "messages": [
                    {"role": "system", "content": "You are a helpful assistant."},
                    {"role": "user", "content": prompt}
                ]
            }),
            json!({"content": response}),
            Some(TokenUsage {
                input_tokens: 20,
                output_tokens: 10,
                total_tokens: 30,
            }),
        );
        
        report_trace(trace);
        
        Ok(response.to_string())
    }
    
    // Create a task that uses the traced LLM
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let response = mock_llm_call(prompt).await?;
            Ok(json!(response))
        }
    });
    
    let cases = vec![
        TestCase::with_id("france", 
            json!("What is the capital of France?"), 
            json!("Paris")),
        TestCase::with_id("germany", 
            json!("What is the capital of Germany?"), 
            json!("Berlin")),
    ];
    let data = Arc::new(VecDataSource::new(cases));
    
    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("capital")),
    ];
    
    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(2)
        .build()?;
    
    let result = eval.run().await?;
    
    // Print summary
    println!("{}", result.summary_table());
    
    // Print traces for each case
    println!("\n=== Detailed Traces ===\n");
    for case_result in &result.cases {
        let id = case_result.case.id.as_deref().unwrap_or("-");
        println!("Case: {}", id);
        println!("Traces: {}", case_result.traces.len());
        
        for (i, trace) in case_result.traces.iter().enumerate() {
            println!("\n  Trace #{}", i + 1);
            if let Some(model) = &trace.model {
                println!("    Model: {}", model);
            }
            if let Some(duration) = trace.duration_ms {
                println!("    Duration: {}ms", duration);
            }
            if let Some(usage) = &trace.usage {
                println!("    Tokens: {} in, {} out, {} total",
                    usage.input_tokens, usage.output_tokens, usage.total_tokens);
            }
            println!("    Input: {}", serde_json::to_string(&trace.input)?);
            println!("    Output: {}", serde_json::to_string(&trace.output)?);
        }
        println!();
    }
    
    // Save with traces to JSON
    let json_output = serde_json::to_string_pretty(&result)?;
    tokio::fs::write("traced_results.json", json_output).await?;
    println!("✓ Full results with traces saved to: traced_results.json");
    
    Ok(())
}




