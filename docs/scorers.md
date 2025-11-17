## Scorers (metrics)

Scorers are responsible for **comparing expected vs output** and producing a `Score`:

- Each scorer implements the async `Scorer` trait:
  - `fn name(&self) -> &'static str`
  - `async fn score(&self, expected: &Value, output: &Value) -> Result<Score>`
- `Score` has:
  - `name: String`
  - `value: f64` (metric value)
  - `passed: bool`
  - `details: Option<serde_json::Value>`

evalcraft ships with several built‑in scorers, and you can implement your own.

---

## Pass / fail semantics

For a given case:

- All scorers in your `Vec<Arc<dyn Scorer>>` are run.
- The case is considered **passed** if:

  \[
  \text{scores is non‑empty} \land \text{all } s.passed \text{ are true}
  \]

- The `passed` column in `summary_table()` and `EvalSummary.passed` use this logic.
- `avg_score` in the table is the mean of `Score.value` across that case’s scorers.

You can mix strict and lenient scorers (e.g. exact + contains) depending on your use case.

---

## Exact match

**Type**: `ExactMatchScorer`  
**Module**: `scorers::exact`

- Compares `expected` and `output` as JSON values:
  - `passed = (expected == output)`
  - `value = 1.0` if equal, else `0.0`.
- Good for:
  - Deterministic functions.
  - Structured outputs where you require an exact match.

---

## Levenshtein (string similarity)

**Type**: `LevenshteinScorer`  
**Module**: `scorers::levenshtein`

- Configuration: `LevenshteinScorer::new(min_similarity: f64)`.
- Logic:
  - Stringify both `expected` and `output`.
  - Compute normalized Levenshtein similarity in \[0, 1\].
  - `passed = (similarity >= min_similarity)`.
  - `value = similarity`.
- Good for:
  - Free‑form text answers where minor differences are OK.
  - Rough similarity scoring.

---

## Contains (substring check)

**Type**: `ContainsScorer`  
**Module**: `scorers::contains`

- Constructors:
  - `ContainsScorer::new(substring)` – case‑sensitive.
  - `ContainsScorer::case_insensitive(substring)` – case‑insensitive.
- Ignores `expected`, only examines `output` as a string:
  - `value = 1.0` if substring is present, else `0.0`.
  - `passed = (value == 1.0)`.
- `details` includes:
  - `substring`, `case_sensitive`, `found`.
- Good for:
  - Factual QA where you only care that key tokens appear.
  - Allowing extra words around the key answer.

---

## Regex

**Type**: `RegexScorer`  
**Module**: `scorers::regex`

- Constructor: `RegexScorer::new(pattern: &str) -> Result<Self>`.
- Ignores `expected`, checks `output` against a regex:
  - `value = 1.0` if `pattern.is_match(output_str)`, else `0.0`.
  - `passed` mirrors `value`.
- `details` includes:
  - `pattern`
  - `matches`
  - `captures` (if any)
- Good for:
  - Format validation (emails, dates, IDs).
  - Enforcing simple structured patterns.

---

## JSON validation and schema

**Type**: `JsonScorer`  
**Module**: `scorers::json`

Modes:

- `JsonScorer::new()` – just ensures output is valid JSON (mostly trivial if you already have `Value`).
- `JsonScorer::with_schema(schema: Value)`:
  - Compiles a JSON Schema and validates `output` against it.
- `JsonScorer::strict()`:
  - Compares the **structure** of `expected` and `output` (keys and types, not values).

Behavior:

- If schema is provided:
  - `passed = true` if schema validation succeeds; `value = 1.0`.
  - Otherwise, `passed = false`, `value = 0.0`, and `details` includes validation errors.
- In strict mode:
  - `passed = true` if structures match, else `false`.
- Default mode:
  - `passed = true`, `value = 1.0` if the value is JSON‑serializable.

Use this for:

- Ensuring structured outputs (e.g. extraction tasks) conform to a contract.
- Separating “shape correctness” from “content correctness”.

---

## SQL validation

**Type**: `SqlScorer`  
**Module**: `scorers::sql`

- Dialects:
  - `SqlScorer::generic()` – most permissive.
  - `SqlScorer::postgres()`
  - `SqlScorer::mysql()`
  - `SqlScorer::sqlite()`
- Behavior:
  - Treats `output` as either:
    - A SQL string, or
    - A JSON object with a `"sql"` string field.
  - Attempts to parse using `sqlparser`.
  - If parse succeeds:
    - `passed = true`, `value = 1.0`.
    - `details` includes statement count, types, and dialect.
  - If parse fails:
    - `passed = false`, `value = 0.0`.
    - `details` includes parse error and dialect.

Use this when:

- You’re testing LLMs that generate SQL.
- You want to guard against invalid syntax before executing queries.

---

## Embedding‑based cosine similarity

**Type**: `EmbeddingScorer`  
**Module**: `scorers::embedding`

This scorer uses **vector embeddings** and **cosine similarity** to assess semantic similarity between `expected` and `output`.

- Configuration:
  - `EmbeddingScorer::new(embed_fn, min_similarity)`.
  - `embed_fn` is an async function/closure that maps `&str` → `Future<Output = Result<Vec<f32>>>`.
- Behavior:
  - Convert `expected` and `output` to strings.
  - Call `embed_fn` on both to get two vectors.
  - Compute cosine similarity in \[0, 1\].
  - `passed = (similarity >= min_similarity)`.
  - `value = similarity`.

`EmbeddingScorer` itself is generic over **how** embeddings are computed. In your application you can:

- Call a localhost embedding server.
- Use a cloud embedding API.
- Use a local model (e.g. via a Rust binding).

See `docs/advanced-llm-eval.md` and `examples/localhost_llm.rs` for a concrete HTTP‑based example.

---

## Implementing a custom scorer

To add your own metric:

1. Create a new module in `crates/evalcraft-core/src/scorers/`, e.g. `my_metric.rs`.
2. Implement `Scorer` with `#[async_trait]`:

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

pub struct MyMetricScorer {
    // config fields here
}

#[async_trait]
impl Scorer for MyMetricScorer {
    fn name(&self) -> &'static str {
        "my_metric"
    }

    async fn score(&self, expected: &Value, output: &Value) -> Result<Score> {
        // compute some f64 score
        let value = 0.0;
        let passed = value > 0.5;

        Ok(Score {
            name: self.name().to_string(),
            value,
            passed,
            details: None,
        })
    }
}
```

3. Re‑export it from `lib.rs` (optional but convenient).
4. Use it like any other scorer by putting it into the `scorers` vector when building an `Eval`.

Because scoring is async, you can safely:

- Call HTTP services.
- Perform filesystem or database I/O.
- Fan out to other internal async operations.
