## Getting started with the library (`evalcraft-core`)

This guide shows how to use **`evalcraft-core` directly from Rust** to evaluate an agent or model.

You’ll:

- Define some test cases.
- Wrap your agent (or any async function) as a `Task`.
- Choose one or more `Scorer`s.
- Run an `Eval` and inspect the results.

---

## 1. Add `evalcraft-core` as a dependency

In your own project’s `Cargo.toml`:

```toml
[dependencies]
anyhow = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"

evalcraft-core = { path = "path/to/this/repo/crates/evalcraft-core" }
```

Adjust the `path` or use `git`/registry coordinates as appropriate.

---

## 2. Define a simple agent and test cases

Example: echo agent that appends `" World!"` and exact‑match scorer:

```rust
use std::sync::Arc;

use anyhow::Result;
use evalcraft_core::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Agent: append " World!" to a string.
    async fn echo_agent(input: &str) -> Result<String> {
        Ok(format!("{input} World!"))
    }

    // Wrap agent as a Task.
    let task = from_async_fn(|input| {
        let input = input.clone();
        async move {
            let s = input.as_str().unwrap_or_default();
            Ok(json!(echo_agent(s).await?))
        }
    });

    // Test cases.
    let cases = vec![
        TestCase::with_id("0", json!("Hello"), json!("Hello World!")),
        TestCase::with_id("1", json!("Hi"), json!("Hi World!")),
    ];
    let data = Arc::new(VecDataSource::new(cases));

    // Scorers.
    let scorers: Vec<Arc<dyn Scorer>> = vec![Arc::new(ExactMatchScorer)];

    // Build and run eval.
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
```

This is essentially the first example in `examples/library_usage.rs`.

---

## 3. Using multiple scorers

You can attach multiple scorers to the same eval. A case is considered “passed” if **all** its scorers pass.

Example: simple QA with contains + Levenshtein:

```rust
let scorers: Vec<Arc<dyn Scorer>> = vec![
    Arc::new(ContainsScorer::case_insensitive("paris")),
    Arc::new(ContainsScorer::case_insensitive("capital")),
    Arc::new(LevenshteinScorer::new(0.6)),
];
```

This setup:

- Ensures the output **mentions the right city** and **context word**.
- Requires overall string similarity ≥ 0.6 to the expected answer.

See `examples/llm_testing_example.rs` for more patterns.

---

## 4. Using JSON and schema validation

For structured outputs (e.g. extraction tasks), use `JsonScorer`:

- Validate that output is valid JSON.
- Optionally, validate against a JSON Schema.

Example (simplified from `library_usage.rs`):

```rust
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
```

This guarantees both **shape** and **exact value matches**.

---

## 5. Semantic scoring with embeddings

For semantic similarity, you can use `EmbeddingScorer`:

- It calls a user‑provided async embedding function.
- It embeds both `expected` and `output`.
- It computes cosine similarity between the two embeddings.

The core crate exposes `EmbeddingScorer`; you pass in an async embedding closure from your application (see `docs/advanced-llm-eval.md` for a full example using a localhost embedding endpoint).

---

## 6. Inspecting results

`EvalResult` gives you both a human‑readable table and programmatic access:

- `result.summary_table()` – text table summary.
- `result.summary` – totals, pass rate, average score.
- `result.cases` – vector of `CaseResult`:
  - `case` – original `TestCase`.
  - `output` – task output.
  - `error` – optional error string, if the task failed.
  - `scores` – vector of `Score` (one per scorer).

You can serialize `EvalResult` to JSON with `serde_json` and save or post‑process it.

---

## 7. Using evals in tests

The `testing` module exposes helpers for writing concise evaluation tests:

- `assert_eval_all_passed(&EvalResult)` – fail if any case failed.
- `assert_eval_pass_rate(&EvalResult, min_pass_rate)` – enforce a minimum pass rate.
- `assert_eval_avg_score(&EvalResult, min_avg_score)` – enforce a minimum average score.

Example (simplified from `examples/test_example.rs`):

```rust
use std::sync::Arc;
use evalcraft_core::*;
use serde_json::json;

#[tokio::test]
async fn test_echo_agent_exact_match() -> anyhow::Result<()> {
    async fn echo_agent(input: &str) -> anyhow::Result<String> {
        Ok(format!("{input} World!"))
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
    assert_eval_all_passed(&result)?;
    Ok(())
}
```

This pattern lets you treat evaluation as just another test, with rich diagnostics when thresholds are not met.

---

## 8. Capturing traces and HTML reports

For LLM‑style tasks, you can capture **per‑call traces** (inputs, outputs, timing, token usage) and later turn them into an HTML report.

Key pieces:

- `Trace` and `TokenUsage` – describe a single model/tool call.
- `report_trace(trace)` – attach a trace to the current case.
- `generate_html_report(&EvalResult)` – turn an `EvalResult` (including traces) into a standalone HTML page.

Example tracing flow (see `examples/tracing_example.rs` and `examples/generate_traced_report.rs`):

1. Wrap your LLM call with a `Trace`:

   ```rust
   async fn mock_llm_call(prompt: &str) -> anyhow::Result<String> {
       let trace_builder = Trace::start_now()
           .model("gpt-4o-mini")
           .id("call-1");

       // ... call your model ...

       let trace = trace_builder.finish(
           json!({ "prompt": prompt }),
           json!({ "response": response }),
           Some(TokenUsage {
               input_tokens: 20,
               output_tokens: 10,
               total_tokens: 30,
           }),
       );

       report_trace(trace);
       Ok(response.to_string())
   }
   ```

2. Run your eval as usual; traces are attached to each `CaseResult`.

3. Serialize the full `EvalResult` to JSON (e.g. `traced_results.json`).

4. In a separate step, generate an HTML report:

   ```rust
   use std::fs;
   use evalcraft_core::{EvalResult, generate_html_report};

   let json = fs::read_to_string("traced_results.json")?;
   let result: EvalResult = serde_json::from_str(&json)?;
   let html = generate_html_report(&result);
   fs::write("traced_report.html", html)?;
   ```

Open `traced_report.html` in a browser to inspect cases, scores, and individual traces interactively.

---

## 9. Persisting results to SQLite

To keep a history of evaluation runs, use the `evalcraft-store` crate:

```rust
use evalcraft_store::Store;

let store = Store::open("eval_history.db")?;
let run_id = store.create_run(Some(json!({
    "environment": "local",
    "user": "example_runner"
})))?;

let eval_id = store.save_eval(run_id, "Capital Cities Eval", &result)?;
println!("Saved eval with id: {}", eval_id);
```

This writes into a small SQLite schema (`runs`, `evals`, `results`, `scores`, `traces`) that you can explore with `sqlite3` or your own tools.  
See `examples/sqlite_persistence.rs` for a complete example, and `docs/getting-started-cli.md` for how the CLI wires this up automatically via `EVALCRAFT_DB_PATH`.

