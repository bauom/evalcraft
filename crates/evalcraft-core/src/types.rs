use serde::{Deserialize, Serialize};
use serde_json::Value;
use tabled::{Table, Tabled};

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

#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
struct SummaryRow {
	id: String,
	passed: String,
	avg_score: f64,
	input: String,
	output: String,
	expected: String,
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
		let rows: Vec<SummaryRow> = self.cases.iter().map(|cr| {
			let id = cr.case.id.clone().unwrap_or_else(|| "-".to_string());
			let passed = if !cr.scores.is_empty() && cr.scores.iter().all(|s| s.passed) { "✓" } else { " " };
			let avg = if cr.scores.is_empty() {
				0.0
			} else {
				let sum: f64 = cr.scores.iter().map(|s| s.value).sum();
				sum / (cr.scores.len() as f64)
			};
			
			SummaryRow {
				id,
				passed: passed.to_string(),
				avg_score: avg,
				input: truncate(value_preview(&cr.case.input), 64),
				output: truncate(value_preview(&cr.output), 64),
				expected: truncate(value_preview(&cr.case.expected), 64),
			}
		}).collect();

		let mut table = Table::new(rows);
		let table_str = table.to_string();

		let summary_text = format!(
			"Total: {}  Passed: {}  Pass rate: {:.1}%  Avg score: {:.3}",
			self.summary.total,
			self.summary.passed,
			self.summary.pass_rate * 100.0,
			self.summary.avg_score
		);

		format!("{}\n\n{}\n", table_str, summary_text)
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



