use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
	pub id: Option<String>,
	pub input: Value,
	pub expected: Value,
}

impl TestCase {
	pub fn new(input: Value, expected: Value) -> Self {
		Self { id: None, input, expected }
	}

	pub fn with_id(id: impl Into<String>, input: Value, expected: Value) -> Self {
		Self { id: Some(id.into()), input, expected }
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
	pub name: String,
	pub value: f64,
	pub passed: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
	pub case: TestCase,
	pub output: Value,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
	pub scores: Vec<Score>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSummary {
	pub total: usize,
	pub passed: usize,
	pub pass_rate: f64,
	pub avg_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
	pub cases: Vec<CaseResult>,
	pub summary: EvalSummary,
}

impl EvalResult {
	pub fn summarize(cases: &[CaseResult]) -> EvalSummary {
		let total = cases.len();
		let mut passed = 0usize;
		let mut score_sum = 0.0f64;
		let mut score_count = 0usize;

		for cr in cases {
			let all_passed = !cr.scores.is_empty() && cr.scores.iter().all(|s| s.passed);
			if all_passed {
				passed += 1;
			}
			for s in &cr.scores {
				score_sum += s.value;
				score_count += 1;
			}
		}

		let pass_rate = if total == 0 { 0.0 } else { passed as f64 / total as f64 };
		let avg_score = if score_count == 0 { 0.0 } else { score_sum / score_count as f64 };

		EvalSummary { total, passed, pass_rate, avg_score }
	}

	pub fn summary_table(&self) -> String {
		// Simple tabular text output; avoid external table crates for minimal deps.
		let mut out = String::new();
		let header = [
			"id",
			"passed",
			"avg_score",
			"input",
			"output",
			"expected",
		];
		out.push_str(&header.join("\t"));
		out.push('\n');
		for cr in &self.cases {
			let id = cr.case.id.clone().unwrap_or_else(|| "-".to_string());
			let passed = if !cr.scores.is_empty() && cr.scores.iter().all(|s| s.passed) { "✓" } else { " " };
			let avg = if cr.scores.is_empty() {
				0.0
			} else {
				let sum: f64 = cr.scores.iter().map(|s| s.value).sum();
				sum / (cr.scores.len() as f64)
			};
			let input = truncate(value_preview(&cr.case.input), 64);
			let output = truncate(value_preview(&cr.output), 64);
			let expected = truncate(value_preview(&cr.case.expected), 64);
			out.push_str(&format!(
				"{}\t{}\t{:.3}\t{}\t{}\t{}\n",
				id, passed, avg, input, output, expected
			));
		}
		out.push('\n');
		out.push_str(&format!(
			"Total: {}  Passed: {}  Pass rate: {:.1}%  Avg score: {:.3}\n",
			self.summary.total,
			self.summary.passed,
			self.summary.pass_rate * 100.0,
			self.summary.avg_score
		));
		out
	}
}

fn value_preview(v: &Value) -> String {
	match v {
		Value::String(s) => s.clone(),
		_ => v.to_string(),
	}
}

fn truncate(s: String, max_len: usize) -> String {
	if s.len() <= max_len {
		return s;
	}
	let mut truncated = s.chars().take(max_len.saturating_sub(1)).collect::<String>();
	truncated.push('…');
	truncated
}


