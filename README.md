## Evalcraft

Evalcraft is a **lightweight evaluation toolkit for agents and LLMs written in Rust**.  
It lets you turn:

- **Data** (test cases in JSON/JSONL or in‑code `TestCase`s)
- **Task** (your agent, function, or HTTP endpoint)
- **Scorers** (string / JSON / SQL / embedding / custom)

into structured **evaluation reports** with per‑case scores, summaries, and optional **traces**.

Evalcraft is split into:

- **`evalcraft-core`** – library crate with the evaluation engine, scorers, tracing, and reporting.
- **`evalcraft-cli`** – command‑line runner that executes your eval tests/examples and wires up persistence.
- **`evalcraft-store`** – small SQLite wrapper for storing `EvalResult`s (runs, scores, traces).
- **`evalcraft-types`** – shared serializable types used by the other crates.

---

## Quick links

- **Concepts & architecture**: see `docs/overview.md`
- **Library quickstart (Rust)**: see `docs/getting-started-library.md`
- **CLI quickstart & watch mode**: see `docs/getting-started-cli.md`
- **Scorers (metrics)**: see `docs/scorers.md`
- **LLM / embeddings, traces & HTML reports**: see `docs/advanced-llm-eval.md`

---

## Installing & building

- **Workspace build**:

```bash
cargo build
```

- **Run core examples**:

```bash
cargo run -p evalcraft-core --example library_usage
cargo run -p evalcraft-core --example llm_testing_example
cargo run -p evalcraft-core --example test_example
cargo run -p evalcraft-core --example tracing_example
```

- **Run the local LLM example** (uses a localhost OpenAI‑compatible server + embedding scorer):

```bash
cargo run -p evalcraft-core --example localhost_llm
```

- **Generate an HTML report from traced results**:

```bash
cargo run -p evalcraft-core --example generate_traced_report
```

- **Build and use the CLI runner**:

```bash
cargo build -p evalcraft-cli
cargo run -p evalcraft-cli -- run           # one‑shot run
cargo run -p evalcraft-cli -- run --watch  # watch mode
```

---

## How it works (very briefly)

- **Data source**: implements the `DataSource` trait and yields a list of `TestCase` values.
- **Task**: implements the async `Task` trait; you can wrap any async function with `from_async_fn`.
- **Scorers**: implement the async `Scorer` trait to compare `expected` vs `output` and emit `Score`s.
- **Runner**: `Eval::builder()` composes data source, task, and scorers, and runs everything concurrently.
- **Traces (optional)**: your task can call `report_trace(...)` to attach rich LLM/tool call traces to each case.

The result is an `EvalResult` with:

- **Per‑case details** (`CaseResult`: input, output, error, scores, traces)  
- **Summary** (`EvalSummary`: total, passed, pass rate, average score)

You can:

- Print `result.summary_table()` in tests or examples.
- Persist runs to SQLite with `evalcraft-store`.
- Render interactive HTML reports using `generate_html_report`.

See the docs linked above for detailed guides and examples.

---

## Documentation index

All detailed docs live in the `docs/` directory:

- `docs/overview.md` – high‑level concepts and architecture.
- `docs/getting-started-library.md` – using `evalcraft-core` directly from Rust.
- `docs/getting-started-cli.md` – using the `evalcraft` CLI to run eval tests/examples and wire SQLite persistence.
- `docs/scorers.md` – built‑in scorers, configuration, and how pass/fail is computed.
- `docs/advanced-llm-eval.md` – HTTP‑based tasks, localhost LLMs, embeddings, traces, and HTML reports.

---

## Contributing

This repo is meant to be **small, readable, and hackable**:

- Most core logic is in `crates/evalcraft-core/src`.
- Examples are in `examples/` and wired via `[[example]]` entries in `Cargo.toml`.

If you add new scorers or features, please:

- Add a small example under `examples/`.
- Extend `docs/scorers.md` and/or `docs/advanced-llm-eval.md` to explain how to use them.


