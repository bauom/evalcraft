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
