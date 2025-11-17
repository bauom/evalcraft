## Getting started with the CLI (`evalcraft-cli`)

The CLI lets you run evaluations **from the command line** using JSONL files and an HTTP task endpoint (or a simple built‑in echo task).

---

## 1. Build the CLI

From the workspace root:

```bash
cargo build -p evalcraft-cli
```

You’ll get a binary named `evalcraft` in `target/debug/` (or `target/release/` if you build with `--release`).

---

## 2. Prepare a JSONL dataset

Create a file (e.g. `examples/openai_tests.jsonl`) where each line is a JSON object with:

- `"id"` (optional): string identifier.
- `"input"`: JSON value (often a string prompt).
- `"expected"`: JSON value (target answer).

Example:

```json
{"id":"capital_france","input":"What is the capital of France?","expected":"Paris"}
{"id":"capital_germany","input":"What is the capital of Germany?","expected":"Berlin"}
{"id":"math","input":"What is 15 + 27?","expected":"42"}
```

---

## 3. Running with the default “echo” task

If you don’t provide an HTTP endpoint, the CLI uses a simple default task:

- For string inputs, it returns `"<input> World!"`.

Example:

```bash
cargo run -p evalcraft-cli -- run \
  --data examples/openai_tests.jsonl \
  --exact
```

Flags used:

- `--data` – path to JSONL file.
- `--exact` – use exact‑match scoring.

---

## 4. Running against an HTTP endpoint (LLM, service, etc.)

Use `--http-url` to specify a task endpoint:

- evalcraft sends either:
  - **POST** with JSON body: `{"input": <value>}` (default), or
  - **GET** with `?input=<json>` query if you set `--http-method GET`.
- It expects a JSON response, which becomes the `output` for scoring.

Example with `POST`:

```bash
cargo run -p evalcraft-cli -- run \
  --data examples/openai_tests.jsonl \
  --http-url http://localhost:8080/chat/completions \
  --http-method POST \
  --contains-i paris \
  --levenshtein 0.6
```

You are responsible for making your endpoint behave the way you expect (e.g. acting as a chat completion API that returns a JSON payload).

---

## 5. Selecting scorers via flags

The CLI exposes a subset of the scorers via flags; you can combine them:

- `--exact` – `ExactMatchScorer`.
- `--levenshtein <min>` – `LevenshteinScorer` with given min similarity.
- `--contains <substring>` – case‑sensitive `ContainsScorer`.
- `--contains-i <substring>` – case‑insensitive `ContainsScorer`.
- `--regex <pattern>` – `RegexScorer`.
- `--json` – `JsonScorer` for JSON validity.
- `--json-schema <file>` – `JsonScorer` with a schema.
- `--sql` – `SqlScorer` for SQL parsing; `--sql-dialect` selects dialect.

If you specify no scorers, the CLI defaults to `ExactMatchScorer`.

Each case is considered “passed” if **all** active scorers pass for that case.

---

## 6. Output formats

By default, the CLI prints a **tabular summary** to stdout:

- Columns: `id`, `passed`, `avg_score`, `input`, `output`, `expected`.
- Footer: `Total`, `Passed`, `Pass rate`, `Avg score`.

You can also dump the full structured result as JSON:

```bash
cargo run -p evalcraft-cli -- run \
  --data examples/openai_tests.jsonl \
  --exact \
  --json-out results.json
```

This writes a `EvalResult` JSON (cases + summary) to `results.json`, which you can post‑process or visualize elsewhere.

---

## 7. When to use CLI vs library

Use the **CLI** when:

- You have a test set as JSONL.
- Your task is exposed via HTTP.
- You want quick experiments and don’t need deep Rust integration.

Use the **library** when:

- You want to integrate evaluation into a Rust codebase.
- You need richer control (custom scorers, embedding calls, complex tasks).
- You need to embed eval runs into automated pipelines or services.

Both share the same concepts (`DataSource`, `Task`, `Scorer`, `Eval`); the CLI is just a thin layer over `evalcraft-core`.
