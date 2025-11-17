## Overview

Evalcraft is a **minimal evaluation SDK for Rust agents and LLMs**, with a small API surface and very few dependencies.
It gives you a small set of primitives:

- **`DataSource`** – where your test cases come from.
- **`Task`** – how you run your agent / model under test.
- **`Scorer`** – how you compare expected vs actual outputs.
- **`Eval`** – orchestration: load cases → run task → apply scorers → summarize.

You can use these either:

- as a **library** (`evalcraft-core`) inside your own Rust project, or  
- via the **CLI** (`evalcraft-cli`) for JSONL‑driven evaluations.

---

## Core concepts

### Test cases

Evalcraft works with generic JSON test cases:

- Each case has:
  - **`id: Option<String>`** – optional identifier.
  - **`input: serde_json::Value`** – arbitrary JSON input.
  - **`expected: serde_json::Value`** – expected output / target.

`TestCase` lives in `types.rs` and is used for both in‑memory and file‑based data sources.

### Data sources

The `DataSource` trait abstracts where cases come from:

- **`VecDataSource`** – simple in‑memory `Vec<TestCase>`.
- **`JsonlDataSource`** – reads a `.jsonl` file where each line is a JSON object with:
  - `"input"` – any JSON value
  - `"expected"` – any JSON value
  - `"id"` (optional) – case ID

You can implement `DataSource` yourself to fetch cases from a DB, API, etc.

### Tasks (what you’re testing)

`Task` represents the thing you’re evaluating:

- Async trait with `async fn run(&self, input: &Value) -> Result<Value>`.
- Typical implementations:
  - A **pure Rust function** (e.g. some transformation).
  - An **HTTP call** to an LLM or service.
  - An **agent** that orchestrates multiple tools.

The helper `from_async_fn` lets you wrap any async closure `Fn(&Value) -> Future<Output = Result<Value>>` into a `Task` without boilerplate.

### Scorers (metrics)

`Scorer` is responsible for comparing `expected` vs `output` and producing a `Score`:

- Async trait with:
  - `fn name(&self) -> &'static str;`
  - `async fn score(&self, expected: &Value, output: &Value) -> Result<Score>;`
- `Score` includes:
  - `name` – metric name (e.g. `"exact_match"`, `"levenshtein"`).
  - `value: f64` – numeric score.
  - `passed: bool` – did this metric pass?
  - `details: Option<Value>` – optional structured metadata.

evalcraft ships with several scorers out of the box (see `docs/scorers.md`).

### Eval runner

`Eval` ties everything together:

- Built via `Eval::builder()` / `EvalBuilder`.
- You configure:
  - `data_source` – any `DataSource`.
  - `task` – any `Task`.
  - `scorers` – list of `Scorer`s.
  - `concurrency` – number of cases to process in parallel.
- `Eval::run().await`:
  - Loads all cases from the data source.
  - Runs the task for each case (concurrently).
  - Applies all scorers to each `(expected, output)` pair (now async‑aware).
  - Produces an `EvalResult` with:
    - `cases: Vec<CaseResult>`
    - `summary: EvalSummary`

`EvalSummary` exposes:

- `total` – number of cases.
- `passed` – number of cases where **all** scorers passed.
- `pass_rate` – ratio in \[0, 1\].
- `avg_score` – average of all `Score.value`s across all cases.

`EvalResult::summary_table()` renders a human‑readable tabular summary suitable for CLI output.

---

## Crates in this repo

### `evalcraft-core` (library)

Provides:

- Traits: `DataSource`, `Task`, `Scorer`.
- Types: `TestCase`, `Score`, `CaseResult`, `EvalSummary`, `EvalResult`.
- Runners: `Eval`, `EvalBuilder`.
- Built‑in scorers: exact match, Levenshtein, contains, regex, JSON validation, SQL validation, embedding‑based.

### `evalcraft-cli` (binary)

Small CLI on top of `evalcraft-core` that:

- Reads test cases from a JSONL file.
- Uses either:
  - a built‑in “echo” task, or
  - an HTTP task endpoint (`--http-url`) for LLMs / remote services.
- Lets you configure scorers via flags (e.g. `--exact`, `--levenshtein`, `--regex`, `--json`, `--sql`, etc.).
- Prints the summary table and optionally writes full results to JSON.

See `docs/getting-started-cli.md` for examples.

---

## Typical usage patterns

- **Local agent eval**:
  - Implement your agent in Rust.
  - Wrap it with `from_async_fn`.
  - Use `VecDataSource` for test cases.
  - Score with `ExactMatchScorer` or `LevenshteinScorer`.

- **HTTP LLM eval**:
  - Use `from_async_fn` to call your LLM over HTTP for each input.
  - Use string/regex scorers (contains, regex) for flexible matching.
  - Optionally add `JsonScorer` for structured outputs.

- **Semantic eval with embeddings**:
  - Implement an `EmbeddingScorer` that calls an embedding endpoint.
  - Use cosine similarity between `expected` and `output` embeddings.
  - Combine with other scorers for robust metrics.

For concrete code examples, see:

- `examples/library_usage.rs`
- `examples/llm_testing_example.rs`
- `examples/openai_example.rs`
- `examples/localhost_llm.rs`


