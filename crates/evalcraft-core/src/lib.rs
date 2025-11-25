//! evalcraft-core: minimal evaluation SDK for Rust agents.
//! Compose data sources, a task (your agent), and scorers; run with concurrency.
//! See `examples/simple.rs` for a quickstart.

pub mod config;
pub mod datasource;
pub mod report;
pub mod runner;
pub mod scorer;
pub mod task;
pub mod testing;
pub mod trace;
pub mod types;

pub mod scorers {
    pub mod contains;
    pub mod embedding;
    pub mod exact;
    pub mod json;
    pub mod levenshtein;
    pub mod regex;
    pub mod sql;
}

pub use config::{EvalConfig, TaskConfig, ScorerConfig, DataConfig};
pub use datasource::{DataSource, JsonlDataSource, VecDataSource};
pub use report::generate_html_report;
pub use runner::{Eval, EvalBuilder};
pub use scorer::Scorer;
pub use scorers::{
    contains::ContainsScorer,
    embedding::EmbeddingScorer,
    exact::ExactMatchScorer,
    json::JsonScorer,
    levenshtein::LevenshteinScorer,
    regex::RegexScorer,
    sql::{SqlDialect, SqlScorer},
};
pub use task::{from_async_fn, Task};
pub use testing::{assert_eval_all_passed, assert_eval_avg_score, assert_eval_pass_rate};
pub use trace::{report_trace, Trace, TokenUsage};
pub use types::{CaseResult, EvalResult, EvalSummary, Score, TestCase};
