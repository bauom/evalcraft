## Getting started with the CLI (`evalcraft-cli`)

The CLI is a **thin wrapper around `cargo test` / examples** that:

- Runs your eval tests and examples.
- Enables the `persistence` feature in `evalcraft-core`.
- Points `evalcraft-core` at a shared SQLite database (`eval_history.db`) for storing results, scores, and traces.

You write your evaluations in Rust (tests or examples) using `evalcraft-core`, and the CLI takes care of:

- Re-running them on code changes (watch mode).
- Wiring up SQLite persistence automatically.

---

## 1. Build the CLI

From the workspace root:

```bash
cargo build -p evalcraft-cli
```

You’ll get a binary named `evalcraft` in `target/debug/` (or `target/release/` if you build with `--release`).

---

## 2. How the CLI works (high level)

The `evalcraft` CLI currently exposes a single subcommand:

```bash
evalcraft run [OPTIONS] [PATH]
```

Under the hood it:

- Runs `cargo test --examples --tests` in the given `PATH` (default: current directory).
- Enables the `evalcraft-core/persistence` feature:
  - This lets `evalcraft-core` write results into a SQLite DB when configured.
- Sets the `EVALCRAFT_DB_PATH` environment variable to `./eval_history.db`:
  - The `evalcraft-store` crate uses this when you open a `Store`.
  - All eval runs share the same DB file by default.

Your Rust tests/examples are responsible for:

- Building `Eval` instances with `evalcraft-core`.
- Optionally using `evalcraft_store::Store` to persist `EvalResult`s, or relying on helper code that does this automatically.

See:

- `examples/test_example.rs` – writing evals as `#[tokio::test]`.
- `examples/sqlite_persistence.rs` – persisting results to SQLite.

---

## 3. Writing evals as tests or examples

You typically:

1. **Use `evalcraft-core` in your crate**:

   ```toml
   [dev-dependencies]
   anyhow = "1"
   tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
   serde_json = "1"
   evalcraft-core = { path = "../crates/evalcraft-core", features = ["persistence"] }
   ```

2. **Write tests or examples** that construct an `Eval`:

   ```rust
   use std::sync::Arc;
   use evalcraft_core::*;
   use serde_json::json;

   #[tokio::test]
   async fn test_my_agent() -> anyhow::Result<()> {
       async fn my_agent(input: &str) -> anyhow::Result<String> {
           Ok(format!("{input} World!"))
       }

       let task = from_async_fn(|input| {
           let input = input.clone();
           async move {
               let s = input.as_str().unwrap_or_default();
               Ok(json!(my_agent(s).await?))
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

       // Use testing helpers to assert behaviour
       assert_eval_all_passed(&result)?;
       Ok(())
   }
   ```

See `examples/test_example.rs` for more patterns (pass-rate assertions, failing evals, etc.).

---

## 4. Running evals once

From your project (or workspace root), run:

```bash
cargo run -p evalcraft-cli -- run
```

This will:

- Run `cargo test --examples --tests --features evalcraft-core/persistence`.
- Create or reuse `eval_history.db` in the current directory.

You can:

- **Filter** by test/example name:

  ```bash
  cargo run -p evalcraft-cli -- run --filter tracing_example
  ```

  This turns into `cargo test --example tracing_example --features evalcraft-core/persistence`.

- **Change the path** to watch/run (defaults to `.`):

  ```bash
  cargo run -p evalcraft-cli -- run ./examples
  ```

---

## 5. Watch mode (auto‑re-run on changes)

Watch mode is useful during development: it re-runs your evals when you change `.rs` or `Cargo.toml` files.

```bash
cargo run -p evalcraft-cli -- run --watch
```

Behavior:

- Watches the given `PATH` (default: current directory).
- On change:
  - If an example/test file changes, runs only the corresponding target.
  - If `Cargo.toml` or shared source changes, runs all tests/examples.
- Always runs with `evalcraft-core/persistence` enabled and `EVALCRAFT_DB_PATH` set.

This lets you iterate quickly on:

- Your agent / task logic.
- Your scorers and thresholds.
- Your test cases.

---

## 6. SQLite persistence (`evalcraft-store`)

The CLI ensures there is an `eval_history.db` SQLite file in the current directory.  
You can use `evalcraft-store` to persist results from within your evaluations:

```rust
use evalcraft_store::Store;

let store = Store::open("eval_history.db")?;
let run_id = store.create_run(Some(json!({
    "environment": "local",
    "user": "example_runner"
})))?;

let eval_id = store.save_eval(run_id, "My Eval Name", &result)?;
println!("Saved eval with id: {}", eval_id);
```

Tables created:

- `runs` – top‑level runs with metadata.
- `evals` – individual evaluations within a run.
- `results` – per‑case inputs/outputs/expected/errors.
- `scores` – per‑case, per‑scorer metrics.
- `traces` – per‑case traces (LLM calls) when you use the tracing system.

See `examples/sqlite_persistence.rs` for a complete, runnable example.

---

## 7. Inspecting historical evals

Because everything is stored in SQLite (`eval_history.db`), you can:

- Inspect data manually:

  ```bash
  sqlite3 eval_history.db '.tables'
  sqlite3 eval_history.db 'SELECT * FROM runs ORDER BY created_at DESC LIMIT 5;'
  ```

- Build custom dashboards or notebooks that read from the DB.
- Combine this with HTML reports (see `examples/generate_traced_report.rs`) to generate rich, browsable reports of past runs.

---

## 8. When to use CLI vs library

Use the **CLI** when:

- You already have or want **Rust tests/examples** for your agents.
- You want **watch mode** and easy re‑runs.
- You want a simple way to **persist results** to SQLite without wiring up commands by hand.

Use the **library** when:

- You’re integrating evals deeply into another Rust service.
- You need custom execution flows, orchestration, or non‑Cargo entrypoints.

Both share the same concepts (`DataSource`, `Task`, `Scorer`, `Eval`, `EvalResult`);  
the CLI just automates running them and wiring persistence.

