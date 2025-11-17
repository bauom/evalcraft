// Example: Testing LLM outputs with Evalcraft
//
// This shows 3 ways to test LLM outputs:
// 1. Using an HTTP endpoint (your LLM API)
// 2. Using a mock LLM for testing the scorers
// 3. Using OpenAI/Anthropic/etc. directly
//
// To run from the workspace root:
//   cargo run -p evalcraft-core --example llm_testing_example

use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== LLM Testing Examples ===\n");

    // Example 1: Mock LLM (for testing)
    println!("1. Testing with Mock LLM (demonstrates scorers)");
    test_with_mock_llm().await?;

    // Example 2: HTTP LLM endpoint
    println!("\n2. Testing with HTTP endpoint (uncomment to use)");
    println!("   See code for how to connect to your LLM API\n");
    // Uncomment to test with your HTTP endpoint:
    // test_with_http_llm().await?;

    // Example 3: Different scorers for different use cases
    println!("3. Using different scorers for different tasks");
    test_different_scoring_strategies().await?;

    Ok(())
}

// Example 1: Mock LLM that returns predefined responses
// Replace this with your actual LLM calls
async fn mock_llm_call(prompt: &str) -> anyhow::Result<String> {
    // Simulate LLM responses
    let response = if prompt.contains("capital of France") {
        "The capital of France is Paris."
    } else if prompt.contains("capital of Germany") {
        "Berlin is the capital of Germany."
    } else if prompt.contains("2 + 2") {
        "The answer is 4"
    } else if prompt.contains("email") {
        "The email address is hello@example.com"
    } else {
        "I don't know"
    };
    
    Ok(response.to_string())
}

async fn test_with_mock_llm() -> anyhow::Result<()> {
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let response = mock_llm_call(prompt).await?;
            Ok(json!(response))
        }
    });

    let cases = vec![
        TestCase::with_id(
            "france",
            json!("What is the capital of France?"),
            json!("Paris")
        ),
        TestCase::with_id(
            "germany",
            json!("What is the capital of Germany?"),
            json!("Berlin")
        ),
        TestCase::with_id(
            "math",
            json!("What is 2 + 2?"),
            json!("4")
        ),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    // Use Contains scorer - good for checking if key info is present
    // even if LLM adds extra explanation
    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("paris")),
        Arc::new(ContainsScorer::case_insensitive("berlin")),
        Arc::new(ContainsScorer::new("4")),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(3)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

// Example 2: Testing with HTTP LLM endpoint
// This shows how to connect to OpenAI, Anthropic, or your own LLM API
#[allow(dead_code)]
async fn test_with_http_llm() -> anyhow::Result<()> {
    // NOTE: You need to add reqwest dependency and set up your API
    // For OpenAI:
    // let api_key = std::env::var("OPENAI_API_KEY")?;
    
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            
            // Example: Call OpenAI API
            // let client = reqwest::Client::new();
            // let response = client
            //     .post("https://api.openai.com/v1/chat/completions")
            //     .header("Authorization", format!("Bearer {}", api_key))
            //     .json(&json!({
            //         "model": "gpt-4o-mini",
            //         "messages": [{"role": "user", "content": prompt}],
            //         "temperature": 0.0
            //     }))
            //     .send()
            //     .await?;
            // let json: serde_json::Value = response.json().await?;
            // let text = json["choices"][0]["message"]["content"].as_str().unwrap_or("");
            // Ok(json!(text))
            
            // For now, just return the prompt as demo
            Ok(json!(format!("LLM response to: {}", prompt)))
        }
    });

    let cases = vec![
        TestCase::with_id(
            "test1",
            json!("What is the capital of France?"),
            json!("Paris")
        ),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("paris")),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(1)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

// Example 3: Different scoring strategies for different tasks
async fn test_different_scoring_strategies() -> anyhow::Result<()> {
    println!("\n--- Factual QA (use Contains) ---");
    test_factual_qa().await?;
    
    println!("\n--- Structured Output (use JSON) ---");
    test_structured_output().await?;
    
    println!("\n--- Format Validation (use Regex) ---");
    test_format_validation().await?;
    
    Ok(())
}

async fn test_factual_qa() -> anyhow::Result<()> {
    // For factual questions, use Contains to check if key info is present
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let prompt = input.as_str().unwrap_or_default();
            let response = if prompt.contains("France") {
                "The capital city of France is Paris, which is also its largest city."
            } else {
                "I don't know"
            };
            Ok(json!(response))
        }
    });

    let cases = vec![
        TestCase::with_id(
            "france",
            json!("What is the capital of France?"),
            json!("Paris")
        ),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(ContainsScorer::case_insensitive("paris")),
        Arc::new(ContainsScorer::case_insensitive("capital")),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(1)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

async fn test_structured_output() -> anyhow::Result<()> {
    // For structured extraction, use JSON schema validation
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let text = input.as_str().unwrap_or_default();
            let response = if text.contains("John") {
                json!({"name": "John Doe", "age": 30})
            } else {
                json!({"name": "Unknown", "age": 0})
            };
            Ok(response)
        }
    });

    let cases = vec![
        TestCase::with_id(
            "john",
            json!("Extract: John Doe is 30 years old"),
            json!({"name": "John Doe", "age": 30})
        ),
    ];
    let data = Arc::new(VecDataSource::new(cases));

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
        Arc::new(ExactMatchScorer), // Also check for exact match
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(1)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

async fn test_format_validation() -> anyhow::Result<()> {
    // For format validation, use Regex
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let text = input.as_str().unwrap_or_default();
            let response = if text.contains("email") {
                "You can reach us at support@example.com"
            } else {
                "No email found"
            };
            Ok(json!(response))
        }
    });

    let cases = vec![
        TestCase::with_id(
            "email",
            json!("Extract the email address"),
            json!("support@example.com")
        ),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    let scorers: Vec<Arc<dyn Scorer>> = vec![
        Arc::new(RegexScorer::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")?),
        Arc::new(ContainsScorer::new("@example.com")),
    ];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(1)
        .build();

    let result = eval.run().await?;
    println!("{}", result.summary_table());
    Ok(())
}

