## Evalcraft

Evalcraft is a **lightweight evaluation toolkit for agents and LLMs written in Rust**.  
It lets you turn:

- **Data** (test cases in JSON/JSONL)
- **Task** (your agent, function, or HTTP endpoint)
- **Scorers** (string / JSON / SQL / embedding / custom)

into structured **evaluation reports** with per‑case scores and summaries.

Evalcraft is split into:

- **`evalcraft-core`** – library crate with the evaluation engine and scorers.
- **`evalcraft-cli`** – command‑line runner built on top of the core.

---

## Quick links

- **Concepts & architecture**: see `docs/overview.md`
- **Library quickstart (Rust)**: see `docs/getting-started-library.md`
- **CLI quickstart**: see `docs/getting-started-cli.md`
- **Scorers (metrics)**: see `docs/scorers.md`
- **LLM / embeddings & advanced usage**: see `docs/advanced-llm-eval.md`

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
cargo run -p evalcraft-core --example openai_example
```

- **Run the local LLM example** (uses a localhost OpenAI‑compatible server + embedding scorer):

```bash
cargo run -p evalcraft-core --example localhost_llm
```

---

## How it works (very briefly)

- **Data source**: implements the `DataSource` trait and yields a list of `TestCase` values.
- **Task**: implements the async `Task` trait; you can wrap any async function with `from_async_fn`.
- **Scorers**: implement the async `Scorer` trait to compare `expected` vs `output` and emit `Score`s.
- **Runner**: `Eval::builder()` composes data source, task, and scorers, and runs everything concurrently.

The result is an `EvalResult` with:

- **Per‑case details** (`CaseResult`: input, output, error, scores)  
- **Summary** (`EvalSummary`: total, passed, pass rate, average score)

See the docs linked above for detailed guides and examples.

---

## Documentation index

All detailed docs live in the `docs/` directory:

- `docs/overview.md` – high‑level concepts and architecture.
- `docs/getting-started-library.md` – using `evalcraft-core` directly from Rust.
- `docs/getting-started-cli.md` – using the `evalcraft` CLI to run evaluations from the shell.
- `docs/scorers.md` – built‑in scorers, configuration, and how pass/fail is computed.
- `docs/advanced-llm-eval.md` – HTTP‑based tasks, localhost LLMs, and embedding‑based semantic scoring.

---

## Contributing

This repo is meant to be **small, readable, and hackable**:

- Most core logic is in `crates/evalcraft-core/src`.
- Examples are in `examples/` and wired via `[[example]]` entries in `Cargo.toml`.

If you add new scorers or features, please:

- Add a small example under `examples/`.
- Extend `docs/scorers.md` and/or `docs/advanced-llm-eval.md` to explain how to use them.


