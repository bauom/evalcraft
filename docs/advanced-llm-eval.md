## Advanced LLM evaluation

This document focuses on **LLM‑centric workflows**:

- Using HTTP endpoints as tasks (LLMs, agents, services).
- Evaluating localhost OpenAI‑compatible servers.
- Semantic scoring with embeddings via `EmbeddingScorer`.
- Capturing detailed **traces** (per‑call metadata) and generating HTML reports.

---

## HTTP‑based tasks (`Task` via `from_async_fn`)

The core pattern is:

- For each test case:
  - Read `input` as a string prompt.
  - Send an HTTP request to your LLM / service.
  - Parse its JSON response as the `output` value.

Basic skeleton:

```rust
use evalcraft_core::*;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;

async fn make_http_task(endpoint: String) -> Arc<dyn Task> {
    let client = Client::new();

    from_async_fn(move |input| {
        let client = client.clone();
        let endpoint = endpoint.clone();
        let input = input.clone();

        async move {
            let prompt = input.as_str().unwrap_or_default();
            let resp = client
                .post(&endpoint)
                .json(&json!({ "input": prompt }))
                .send()
                .await?;
            let v = resp.json::<serde_json::Value>().await?;
            Ok(v)
        }
    })
}
```

You can adapt this to:

- Chat‑style APIs (with `"messages"` + `"model"`).
- OpenAI / OpenAI‑compatible servers.
- Any custom JSON protocol.

---

## Localhost LLM example (`examples/localhost_llm.rs`)

The repository includes an example that demonstrates:

- Calling a **localhost OpenAI‑compatible LLM** for each test case.
- Using a **system prompt** to steer the model toward concise 1‑word answers.
- Scoring with both **Levenshtein** and **embedding‑based** metrics.

High‑level flow:

1. Read test cases from `examples/openai_tests.jsonl`.
2. For each case:
   - Build a `messages` array with a system message + user prompt.
   - Call a chat completions endpoint at `http://localhost:8080/chat/completions`.
   - Extract the model’s answer as `output`.
3. Compute:
   - Levenshtein similarity to the expected answer.
   - Embedding cosine similarity using a separate `/embeddings` endpoint.
4. Print a table and detailed per‑scorer scores.

You can use this as a template and swap out:

- Endpoint URLs.
- Model names.
- System prompts.
- Scorer thresholds.

---

## Embedding‑based semantic scoring

### Why embeddings?

Levenshtein and string‑based scorers care about **surface form**. They often fail to recognize:

- Synonyms.
- Paraphrases.
- Different word orders.

Embeddings map text to **vectors in a semantic space**. Cosine similarity between these vectors captures:

- “Same meaning in different words.”
- High semantic overlap even with different phrasing.

### `EmbeddingScorer` in core

`EmbeddingScorer` in `evalcraft-core` is designed to be **embedding‑provider agnostic**:

- You supply an async function/closure:

  ```rust
  type EmbedFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;

  Arc<dyn for<'a> Fn(&'a str) -> EmbedFuture<'a> + Send + Sync>
  ```

- `EmbeddingScorer`:
  - Calls your function for `expected` and `output`.
  - Computes cosine similarity in \[0, 1\].
  - Marks passed if similarity ≥ `min_similarity`.

See `crates/evalcraft-core/src/scorers/embedding.rs` for the implementation details.

---

## Wiring embeddings to a localhost endpoint

Suppose you have an embeddings endpoint that behaves like your `curl` example:

```bash
curl -X POST http://localhost:8080/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input": "What is the capital of France?"}'
```

Response shape (simplified):

```json
[
  {
    "index": 0,
    "embedding": [[11.53, 12.30, 10.62, ...]]
  }
]
```

You can build an embedding function like this (see `examples/localhost_llm.rs` for the full context):

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use evalcraft_core::EmbeddingScorer;
use reqwest::Client;
use serde_json::json;

type EmbedFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;

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

            // Expect: [{"index":0,"embedding":[[...]]}]
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
```

You then pass this into `EmbeddingScorer`:

```rust
let scorers: Vec<Arc<dyn Scorer>> = vec![
    Arc::new(LevenshteinScorer::new(0.6)),
    Arc::new(EmbeddingScorer::new(make_embed_fn(), 0.6)),
];
```

Now each case will get both:

- A **string similarity** score.
- A **semantic similarity** score.

---

## Traces: capturing per‑call metadata

For LLM‑heavy workloads, it’s often useful to capture more than just final outputs:

- Prompt and response JSON.
- Model name and latency.
- Token usage (input/output/total).

Evalcraft’s tracing system (`trace` module) provides:

- `Trace` – a serializable record of a single model/tool call.
- `TokenUsage` – token accounting.
- `Trace::start_now().model(...).id(...)` – builder API for timed traces.
- `report_trace(trace)` – attach a trace to the **current case** while the eval is running.

When you call `report_trace`, traces end up in `CaseResult.traces`, and are:

- Persisted by `evalcraft-store` (into the `traces` table).
- Rendered by `generate_html_report` in the HTML report.

See `examples/tracing_example.rs` for a complete, runnable demo.

---

## Combining scorers for robust evaluation

For LLM QA tasks, a practical combo is:

- **Contains or regex**:
  - Check that key tokens (e.g. `"paris"`, `"capital"`) are present.
  - Avoid failing due to extra context sentences.
- **Levenshtein**:
  - Basic sanity check on text closeness.
- **EmbeddingScorer**:
  - Semantic similarity in case the wording is very different.
- **JsonScorer**:
  - When you ask the model to output structured JSON.
- **SqlScorer**:
  - When you ask the model to generate SQL.

You can tune each scorer’s threshold to match your tolerance for errors and verbosity.

---

## Debugging and inspecting results

To understand how your LLM and scorers behave:

- Start with **few test cases** and **print detailed scores** per case (as done in `localhost_llm.rs`).
- Inspect:
  - `Score.value` for each scorer.
  - Whether `passed` is what you expect.
  - `EvalSummary.pass_rate` and `avg_score`.
- Inspect **traces** (if you use `report_trace`) to see:
  - Exact prompts and structured inputs you sent.
  - Raw model responses.
  - Latency and token usage.
- Adjust thresholds, prompts, and scorers based on what you consider “acceptable” model behavior.

Because `EvalResult` is serializable, you can:

- Save it as JSON.
- Feed it into notebooks, dashboards, or visualization tools.
- Render it as an interactive HTML report via `generate_html_report`.
- Persist it (and traces) into SQLite with `evalcraft-store` for long‑term analysis.


