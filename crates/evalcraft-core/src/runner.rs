use std::sync::Arc;

use anyhow::Result;
use futures::stream::{self, StreamExt};

use crate::datasource::DataSource;
use crate::scorer::Scorer;
use crate::task::Task;
use crate::types::{CaseResult, EvalResult, TestCase};

pub struct EvalBuilder {
	data_source: Option<Arc<dyn DataSource>>,
	task: Option<Arc<dyn Task>>,
	scorers: Vec<Arc<dyn Scorer>>,
	concurrency: usize,
}

impl EvalBuilder {
	pub fn new() -> Self {
		Self {
			data_source: None,
			task: None,
			scorers: Vec::new(),
			concurrency: 8,
		}
	}

	pub fn data_source(mut self, data_source: Arc<dyn DataSource>) -> Self {
		self.data_source = Some(data_source);
		self
	}

	pub fn task(mut self, task: Arc<dyn Task>) -> Self {
		self.task = Some(task);
		self
	}

	pub fn scorers<I>(mut self, scorers: I) -> Self
	where
		I: IntoIterator<Item = Arc<dyn Scorer>>,
	{
		self.scorers = scorers.into_iter().collect();
		self
	}

	pub fn add_scorer(mut self, scorer: Arc<dyn Scorer>) -> Self {
		self.scorers.push(scorer);
		self
	}

	pub fn concurrency(mut self, n: usize) -> Self {
		self.concurrency = n.max(1);
		self
	}

	pub fn build(self) -> Result<Eval> {
		Ok(Eval {
			data_source: self.data_source.ok_or_else(|| anyhow::anyhow!("data_source must be set"))?,
			task: self.task.ok_or_else(|| anyhow::anyhow!("task must be set"))?,
			scorers: self.scorers,
			concurrency: self.concurrency,
		})
	}
}

pub struct Eval {
	data_source: Arc<dyn DataSource>,
	task: Arc<dyn Task>,
	scorers: Vec<Arc<dyn Scorer>>,
	concurrency: usize,
}

impl Eval {
	pub fn builder() -> EvalBuilder {
		EvalBuilder::new()
	}

	pub async fn run(&self) -> Result<EvalResult> {
		let cases = self.data_source.load().await?;
		let results = self.run_cases(cases).await?;
		let summary = crate::types::EvalResult::summarize(&results);
		let result = EvalResult { cases: results, summary };

		// Implicit Persistence:
		// If running under `evalcraft run`, this environment variable will be set.
		// We automatically save the run to the database.
		if let Ok(db_path) = std::env::var("EVALCRAFT_DB_PATH") {
			#[cfg(feature = "persistence")]
			{
                // We use core::task::spawn_blocking or just run it if store is sync?
                // Store::open is synchronous (rusqlite).
                // We are in async context.
                let result_clone = result.clone(); // Expensive clone? EvalResult can be large.
                // Ideally we save refs, but `spawn_blocking` needs 'static.
                // Let's just do it synchronously for now as it's local DB and end of run.
                
                match evalcraft_store::Store::open(&db_path) {
                    Ok(store) => {
                        // Create a run for this specific eval execution
                        // In a real scenario, the CLI might create the "Run" and pass the ID via env var.
                        // But for now, let's create a run per eval file execution.
                         match store.create_run(Some(serde_json::json!({
                            "source": "implicit_persistence",
                            "eval_type": "auto"
                        }))) {
                            Ok(run_id) => {
                                if let Err(e) = store.save_eval(run_id, "Auto Eval", &result_clone) {
                                    eprintln!("Failed to save eval results to store: {}", e);
                                }
                            }
                            Err(e) => eprintln!("Failed to create run in store: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to open store at {}: {}", db_path, e),
                }
			}
			#[cfg(not(feature = "persistence"))]
			{
				eprintln!("Info: EVALCRAFT_DB_PATH set to {}, but 'persistence' feature is not enabled. Results will not be persisted.", db_path);
			}
		}

		Ok(result)
	}

	/// Run only the task for all cases, skipping scorers.
	/// Useful for generating goldens or debugging traces/outputs.
	pub async fn run_without_scoring(&self) -> Result<EvalResult> {
		let cases = self.data_source.load().await?;
		let results = self.run_cases_internal(cases, false).await?;
		// Summary will show 0 scores but correct pass/fail based on errors
		let summary = crate::types::EvalResult::summarize(&results);
		Ok(EvalResult { cases: results, summary })
	}

	async fn run_cases(&self, cases: Vec<TestCase>) -> Result<Vec<CaseResult>> {
		self.run_cases_internal(cases, true).await
	}

	async fn run_cases_internal(&self, cases: Vec<TestCase>, run_scorers: bool) -> Result<Vec<CaseResult>> {
		let task = self.task.clone();
		let scorers = if run_scorers { self.scorers.clone() } else { Vec::new() };
		
		let stream = stream::iter(cases.into_iter()).map(move |case| {
			let task = task.clone();
			let scorers = scorers.clone();
			async move {
				let (execution_result, traces) = crate::trace::scope_traces(async {
					match task.run(&case.input).await {
						Ok(output) => {
							let mut scores = Vec::with_capacity(scorers.len());
							if !scorers.is_empty() {
								for s in &scorers {
									match s.score(&case.expected, &output).await {
										Ok(score) => scores.push(score),
										Err(err) => scores.push(crate::types::Score {
											name: s.name().to_string(),
											value: 0.0,
											passed: false,
											details: Some(serde_json::json!({ "error": err.to_string() })),
										}),
									}
								}
							}
							Ok((output, scores))
						}
						Err(e) => Err(e),
					}
				}).await;

				match execution_result {
					Ok((output, scores)) => CaseResult {
						case,
						output,
						error: None,
						scores,
						traces,
					},
					Err(err) => CaseResult {
						case,
						output: serde_json::Value::Null,
						error: Some(err.to_string()),
						scores: Vec::new(),
						traces,
					},
				}
			}
		});

		let results: Vec<CaseResult> = stream
			.buffer_unordered(self.concurrency)
			.collect()
			.await;
		Ok(results)
	}
}


