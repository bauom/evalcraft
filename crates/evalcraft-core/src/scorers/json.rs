use anyhow::Result;
use async_trait::async_trait;
use jsonschema::JSONSchema;
use serde_json::Value;

use crate::scorer::Scorer;
use crate::types::Score;

/// Validates JSON structure and optionally checks against a JSON schema.
pub struct JsonScorer {
	schema: Option<JSONSchema>,
	strict: bool,
}

impl JsonScorer {
	/// Creates a JSON scorer that only validates if output is valid JSON.
	pub fn new() -> Self {
		Self {
			schema: None,
			strict: false,
		}
	}

	/// Creates a JSON scorer with a schema for validation.
	/// Returns error if schema is invalid.
	pub fn with_schema(schema: Value) -> Result<Self> {
		let compiled = JSONSchema::compile(&schema)
			.map_err(|e| anyhow::anyhow!("Invalid JSON schema: {}", e))?;
		Ok(Self {
			schema: Some(compiled),
			strict: false,
		})
	}

	/// Creates a JSON scorer that requires exact structural match with expected.
	pub fn strict() -> Self {
		Self {
			schema: None,
			strict: true,
		}
	}
}

impl Default for JsonScorer {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Scorer for JsonScorer {
	fn name(&self) -> &'static str {
		"json"
	}

	async fn score(&self, expected: &Value, output: &Value) -> Result<Score> {
		// Check if output is valid JSON (always true if we got here with Value)
		// But we validate it can be serialized/deserialized
		let output_str = serde_json::to_string(output)?;
		let parsed: Value = serde_json::from_str(&output_str)?;

		// If schema is provided, validate against it
		if let Some(ref schema) = self.schema {
			match schema.validate(&parsed) {
				Ok(_) => {
					return Ok(Score {
						name: self.name().to_string(),
						value: 1.0,
						passed: true,
						details: Some(serde_json::json!({
							"valid": true,
							"message": "Output matches JSON schema"
						})),
					});
				}
				Err(errors) => {
					let error_msgs: Vec<String> = errors
						.map(|e| format!("{}: {}", e.instance_path, e))
						.collect();
					return Ok(Score {
						name: self.name().to_string(),
						value: 0.0,
						passed: false,
						details: Some(serde_json::json!({
							"valid": false,
							"errors": error_msgs
						})),
					});
				}
			}
		}

		// If strict mode, check for exact structural match
		if self.strict {
			let structures_match = compare_structure(expected, &parsed);
			return Ok(Score {
				name: self.name().to_string(),
				value: if structures_match { 1.0 } else { 0.0 },
				passed: structures_match,
				details: Some(serde_json::json!({
					"strict": true,
					"structures_match": structures_match
				})),
			});
		}

		// Default: just validate it's valid JSON
		Ok(Score {
			name: self.name().to_string(),
			value: 1.0,
			passed: true,
			details: Some(serde_json::json!({
				"valid": true,
				"message": "Valid JSON"
			})),
		})
	}
}

/// Recursively compare JSON structure (keys and types, not values)
fn compare_structure(expected: &Value, actual: &Value) -> bool {
	match (expected, actual) {
		(Value::Object(e), Value::Object(a)) => {
			if e.len() != a.len() {
				return false;
			}
			for (key, e_val) in e.iter() {
				match a.get(key) {
					Some(a_val) => {
						if !compare_structure(e_val, a_val) {
							return false;
						}
					}
					None => return false,
				}
			}
			true
		}
		(Value::Array(e), Value::Array(a)) => {
			if e.len() != a.len() {
				return false;
			}
			e.iter()
				.zip(a.iter())
				.all(|(e_item, a_item)| compare_structure(e_item, a_item))
		}
		(Value::String(_), Value::String(_)) => true,
		(Value::Number(_), Value::Number(_)) => true,
		(Value::Bool(_), Value::Bool(_)) => true,
		(Value::Null, Value::Null) => true,
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_json_valid() {
		let scorer = JsonScorer::new();
		let output = serde_json::json!({"name": "John", "age": 30});
		let expected = serde_json::json!({});
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}

	#[test]
	fn test_json_strict_match() {
		let scorer = JsonScorer::strict();
		let expected = serde_json::json!({"name": "John", "age": 30});
		let output = serde_json::json!({"name": "Jane", "age": 25});
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed); // Structure matches, values differ
	}

	#[test]
	fn test_json_strict_mismatch() {
		let scorer = JsonScorer::strict();
		let expected = serde_json::json!({"name": "John", "age": 30});
		let output = serde_json::json!({"name": "Jane"}); // Missing "age"
		let score = scorer.score(&expected, &output).unwrap();
		assert!(!score.passed);
		assert_eq!(score.score, 0.0);
	}

	#[test]
	fn test_json_with_schema() {
		let schema = serde_json::json!({
			"type": "object",
			"properties": {
				"name": {"type": "string"},
				"age": {"type": "number"}
			},
			"required": ["name", "age"]
		});
		let scorer = JsonScorer::with_schema(schema).unwrap();
		let output = serde_json::json!({"name": "John", "age": 30});
		let expected = serde_json::json!({});
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}

	#[test]
	fn test_json_schema_fail() {
		let schema = serde_json::json!({
			"type": "object",
			"properties": {
				"name": {"type": "string"},
				"age": {"type": "number"}
			},
			"required": ["name", "age"]
		});
		let scorer = JsonScorer::with_schema(schema).unwrap();
		let output = serde_json::json!({"name": "John"}); // Missing age
		let expected = serde_json::json!({});
		let score = scorer.score(&expected, &output).unwrap();
		assert!(!score.passed);
		assert_eq!(score.score, 0.0);
	}
}

