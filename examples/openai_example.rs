// Example: Testing OpenAI with Evalcraft
//
// This is a minimal, production-ready example showing how to test OpenAI outputs.
//
// Setup:
// 1. Add to your Cargo.toml:
//    async-openai = "0.23"
//    tokio = { version = "1", features = ["full"] }
//    evalcraft-core = { path = "../crates/evalcraft-core" }
//
// 2. Set your API key:
//    export OPENAI_API_KEY="sk-..."
//
// 3. Run from the workspace root:
//    cargo run -p evalcraft-core --example openai_example

use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use evalcraft_core::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Check API key is set
    std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    println!("Testing OpenAI with Evalcraft\n");

    // Run the evaluation
    run_openai_eval().await?;

    Ok(())
}

async fn run_openai_eval() -> anyhow::Result<()> {
    // Define the task: call OpenAI for each input
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let response = call_openai(prompt).await?;
            Ok(json!(response))
        }
    });

    // Load test cases from JSONL file
    let data = Arc::new(JsonlDataSource::new("examples/openai_tests.jsonl"));

    // Define scorers - use Contains for flexible matching
    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("paris")),
        Arc::new(ContainsScorer::case_insensitive("berlin")),
        Arc::new(LevenshteinScorer::new(0.6)), // 60% similarity
    ];

    // Build and run evaluation
    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(3) // Respect OpenAI rate limits
        .build()?;

    let result = eval.run().await?;

    // Print results
    println!("{}", result.summary_table());

    // Optionally save to JSON
    let json_result = serde_json::to_string_pretty(&result)?;
    tokio::fs::write("openai_results.json", json_result).await?;
    println!("\nResults saved to openai_results.json");

    Ok(())
}

/// Call OpenAI API
async fn call_openai(prompt: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let messages = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful assistant. Answer concisely.")
                .build()?,
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt)
                .build()?,
        ),
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini") // Fast and cheap for testing
        .messages(messages)
        .temperature(0.0) // Deterministic for testing
        .max_tokens(100u32)
        .build()?;

    let response = client.chat().create(request).await?;

    let content = response.choices[0]
        .message
        .content
        .clone()
        .unwrap_or_default();

    Ok(content)
}
