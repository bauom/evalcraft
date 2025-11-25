// Example: Integration with cargo test
//
// Run this with: cargo test --example test_example
//
// This shows how to write evaluations as regular Rust tests.

use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;

// This function is needed for the example to be compilable as a binary (cargo run --example test_example)
// but when running `cargo test`, this main is ignored.
fn main() {
    println!("Run this example with: cargo test --example test_example");
}

#[tokio::test]
async fn test_echo_agent_exact_match() -> anyhow::Result<()> {
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
    
    // Use the helper to assert all tests passed
    assert_eval_all_passed(&result)?;
    
    Ok(())
}

#[tokio::test]
async fn test_echo_agent_pass_rate() -> anyhow::Result<()> {
    // Agent that sometimes fails
    async fn buggy_agent(input: &str) -> anyhow::Result<String> {
        if input.contains("fail") {
            Ok("Wrong".to_string())
        } else {
            Ok(format!("{} World!", input))
        }
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(buggy_agent(s).await?))
        }
    });

    let cases = vec![
        TestCase::with_id("0", json!("Hello"), json!("Hello World!")),
        TestCase::with_id("1", json!("fail"), json!("fail World!")), // This will fail
        TestCase::with_id("2", json!("Hi"), json!("Hi World!")),
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
    
    // Assert at least 60% pass rate (2/3 = 66%)
    assert_eval_pass_rate(&result, 0.6)?;
    
    Ok(())
}

#[tokio::test]
#[should_panic(expected = "pass rate")]
async fn test_failing_eval() {
    // This test demonstrates that failing evals panic (as expected in tests)
    async fn bad_agent(_input: &str) -> anyhow::Result<String> {
        Ok("Always wrong".to_string())
    }

    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(bad_agent(s).await?))
        }
    });

    let cases = vec![
        TestCase::with_id("0", json!("Hello"), json!("Hello World!")),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    let scorers: Vec<Arc<dyn Scorer>> = vec![Arc::new(ExactMatchScorer)];

    let eval = Eval::builder()
        .data_source(data)
        .task(task)
        .scorers(scorers)
        .concurrency(4)
        .build()
        .unwrap();

    let result = eval.run().await.unwrap();
    
    // This will panic because pass rate is 0%
    assert_eval_pass_rate(&result, 0.8).unwrap();
}


