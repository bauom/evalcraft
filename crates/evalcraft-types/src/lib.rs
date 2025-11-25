use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;
use tabled::Tabled;

/// Trace data for a single LLM or API call within a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// Unique identifier for this trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// When the call started
    pub start: SystemTime,
    
    /// When the call ended
    pub end: SystemTime,
    
    /// Duration of the call in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    
    /// Model name (e.g., "gpt-4o-mini")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// Input to the model (messages, prompt, etc.)
    pub input: serde_json::Value,
    
    /// Output from the model
    pub output: serde_json::Value,
    
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    
    /// Additional metadata (provider, temperature, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    
    /// Error if the call failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl Trace {
    /// Create a new trace with start time
    pub fn start_now() -> TraceBuilder {
        TraceBuilder {
            start: SystemTime::now(),
            id: None,
            model: None,
            metadata: None,
        }
    }
}

/// Builder for creating traces
pub struct TraceBuilder {
    start: SystemTime,
    id: Option<String>,
    model: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl TraceBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
    
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    pub fn finish(
        self,
        input: serde_json::Value,
        output: serde_json::Value,
        usage: Option<TokenUsage>,
    ) -> Trace {
        let end = SystemTime::now();
        let duration_ms = end
            .duration_since(self.start)
            .ok()
            .map(|d| d.as_millis() as u64);
        
        Trace {
            id: self.id,
            start: self.start,
            end,
            duration_ms,
            model: self.model,
            input,
            output,
            usage,
            metadata: self.metadata,
            error: None,
        }
    }
    
    pub fn finish_with_error(
        self,
        input: serde_json::Value,
        error: String,
    ) -> Trace {
        let end = SystemTime::now();
        let duration_ms = end
            .duration_since(self.start)
            .ok()
            .map(|d| d.as_millis() as u64);
        
        Trace {
            id: self.id,
            start: self.start,
            end,
            duration_ms,
            model: self.model,
            input,
            output: serde_json::Value::Null,
            usage: None,
            metadata: self.metadata,
            error: Some(error),
        }
    }
}

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
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	pub traces: Vec<Trace>,
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
        use tabled::Table;
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

		let table = Table::new(rows);
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
