// Example: Using Evalcraft core library directly with your agent
//
// This shows how to integrate all the scorers into your Rust code
// instead of using the CLI.
//
// To run from the workspace root:
//   cargo run -p evalcraft-core --example library_usage

use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Example 1: Simple agent with exact match
    println!("=== Example 1: Exact Match ===\n");
    run_exact_match_example().await?;

    println!("\n=== Example 2: Multiple Scorers ===\n");
    run_multiple_scorers_example().await?;

    println!("\n=== Example 3: JSON Validation ===\n");
    run_json_validation_example().await?;

    println!("\n=== Example 4: Regex Pattern Matching ===\n");
    run_regex_example().await?;

    Ok(())
}

// Example 1: Exact match scorer
async fn run_exact_match_example() -> anyhow::Result<()> {
    // Your agent function
    async fn echo_agent(input: &str) -> anyhow::Result<String> {
        Ok(format!("{} World!", input))
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(echo_agent(s).await?))
        }
    });

    let cases = vec![
        TestCase::with_id("0", json!("Hello"), json!("Hello World!")),
        TestCase::with_id("1", json!("Hi"), json!("Hi World!")),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    let scorers: Vec<Arc<dyn Scorer>> = vec![Arc::new(ExactMatchScorer)];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(4)
        .build()?;

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

// Example 2: Multiple scorers (all must pass)
async fn run_multiple_scorers_example() -> anyhow::Result<()> {
    // Agent that extracts capitals
    async fn capital_agent(question: &str) -> anyhow::Result<String> {
        // In real app, this would call your LLM
        let response = if question.contains("France") {
            "The capital of France is Paris"
        } else if question.contains("Germany") {
            "The capital of Germany is Berlin"
        } else {
            "I don't know"
        };
        Ok(response.to_string())
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(capital_agent(s).await?))
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
        Arc::new(ContainsScorer::case_insensitive("paris")),
        Arc::new(LevenshteinScorer::new(0.5)),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(2)
        .build()?;

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

// Example 3: JSON validation with schema
async fn run_json_validation_example() -> anyhow::Result<()> {
    // Agent that extracts structured data
    async fn extraction_agent(text: &str) -> anyhow::Result<serde_json::Value> {
        // In real app, this would call your LLM
        if text.contains("John") {
            Ok(json!({
                "name": "John Doe",
                "age": 30
            }))
        } else {
            Ok(json!({
                "name": "Jane Smith",
                "age": 25
            }))
        }
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            extraction_agent(s).await
        }
    });

    let cases = vec![
        TestCase::with_id("john",
            json!("Extract info: John Doe is 30 years old"),
            json!({"name": "John Doe", "age": 30})),
        TestCase::with_id("jane",
            json!("Extract info: Jane Smith is 25 years old"),
            json!({"name": "Jane Smith", "age": 25})),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    // Define JSON schema
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "number"}
        },
        "required": ["name", "age"]
    });

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(JsonScorer::with_schema(schema)?),
        Arc::new(ExactMatchScorer),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(2)
        .build()?;

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

// Example 4: Regex pattern validation
async fn run_regex_example() -> anyhow::Result<()> {
    // Agent that extracts emails
    async fn email_extractor(text: &str) -> anyhow::Result<String> {
        // In real app, this would use your LLM
        if text.contains("support") {
            Ok("support@example.com".to_string())
        } else if text.contains("sales") {
            Ok("sales@example.com".to_string())
        } else {
            Ok("info@example.com".to_string())
        }
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(email_extractor(s).await?))
        }
    });

    let cases = vec![
        TestCase::with_id("support",
            json!("Contact support at support@example.com"),
            json!("support@example.com")),
        TestCase::with_id("sales",
            json!("Email sales team: sales@example.com"),
            json!("sales@example.com")),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    // Email regex pattern
    let email_pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(RegexScorer::new(email_pattern)?),
        Arc::new(ExactMatchScorer),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(2)
        .build()?;

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

