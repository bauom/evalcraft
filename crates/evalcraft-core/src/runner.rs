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
		Ok(EvalResult { cases: results, summary })
	}

	async fn run_cases(&self, cases: Vec<TestCase>) -> Result<Vec<CaseResult>> {
		let task = self.task.clone();
		let scorers = self.scorers.clone();
		let stream = stream::iter(cases.into_iter()).map(move |case| {
			let task = task.clone();
			let scorers = scorers.clone();
			async move {
				match task.run(&case.input).await {
					Ok(output) => {
						let mut scores = Vec::with_capacity(scorers.len());
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
						CaseResult {
							case,
							output,
							error: None,
							scores,
						}
					}
					Err(err) => {
						CaseResult {
							case,
							output: serde_json::Value::Null,
							error: Some(err.to_string()),
							scores: Vec::new(),
						}
					}
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


