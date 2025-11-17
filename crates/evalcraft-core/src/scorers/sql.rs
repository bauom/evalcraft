use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlparser::ast::Statement;
use sqlparser::dialect::{Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

use crate::scorer::Scorer;
use crate::types::Score;

/// SQL dialect to use for parsing.
#[derive(Debug, Clone, Copy)]
pub enum SqlDialect {
	Generic,
	PostgreSQL,
	MySQL,
	SQLite,
}

impl SqlDialect {
	fn to_dialect(&self) -> Box<dyn Dialect> {
		match self {
			SqlDialect::Generic => Box::new(GenericDialect {}),
			SqlDialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
			SqlDialect::MySQL => Box::new(MySqlDialect {}),
			SqlDialect::SQLite => Box::new(SQLiteDialect {}),
		}
	}
}

/// Validates SQL syntax using sqlparser.
pub struct SqlScorer {
	dialect: SqlDialect,
}

impl SqlScorer {
	/// Creates a SQL scorer with the given dialect.
	pub fn new(dialect: SqlDialect) -> Self {
		Self { dialect }
	}

	/// Creates a SQL scorer with generic SQL dialect (most permissive).
	pub fn generic() -> Self {
		Self::new(SqlDialect::Generic)
	}

	/// Creates a SQL scorer for PostgreSQL.
	pub fn postgres() -> Self {
		Self::new(SqlDialect::PostgreSQL)
	}

	/// Creates a SQL scorer for MySQL.
	pub fn mysql() -> Self {
		Self::new(SqlDialect::MySQL)
	}

	/// Creates a SQL scorer for SQLite.
	pub fn sqlite() -> Self {
		Self::new(SqlDialect::SQLite)
	}
}

impl Default for SqlScorer {
	fn default() -> Self {
		Self::generic()
	}
}

#[async_trait]
impl Scorer for SqlScorer {
	fn name(&self) -> &'static str {
		"sql"
	}

	async fn score(&self, _expected: &Value, output: &Value) -> Result<Score> {
		let sql_str = match output {
			Value::String(s) => s.clone(),
			_ => {
				// Try to extract SQL from JSON object
				if let Some(sql) = output.get("sql").and_then(|v| v.as_str()) {
					sql.to_string()
				} else {
					serde_json::to_string(output)?
				}
			}
		};

		let dialect = self.dialect.to_dialect();

		match Parser::parse_sql(&*dialect, &sql_str) {
			Ok(statements) => {
				let statement_types: Vec<String> = statements
					.iter()
					.map(|stmt| match stmt {
						Statement::Query(_) => "SELECT".to_string(),
						Statement::Insert { .. } => "INSERT".to_string(),
						Statement::Update { .. } => "UPDATE".to_string(),
						Statement::Delete { .. } => "DELETE".to_string(),
						Statement::CreateTable { .. } => "CREATE TABLE".to_string(),
						Statement::AlterTable { .. } => "ALTER TABLE".to_string(),
						Statement::Drop { .. } => "DROP".to_string(),
						_ => "OTHER".to_string(),
					})
					.collect();

				Ok(Score {
					name: self.name().to_string(),
					value: 1.0,
					passed: true,
					details: Some(serde_json::json!({
						"valid": true,
						"statement_count": statements.len(),
						"statement_types": statement_types,
						"dialect": format!("{:?}", self.dialect)
					})),
				})
			}
			Err(e) => Ok(Score {
				name: self.name().to_string(),
				value: 0.0,
				passed: false,
				details: Some(serde_json::json!({
					"valid": false,
					"error": e.to_string(),
					"dialect": format!("{:?}", self.dialect)
				})),
			}),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_sql_valid_select() {
		let scorer = SqlScorer::generic();
		let output = serde_json::json!("SELECT * FROM users WHERE age > 18");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}

	#[test]
	fn test_sql_valid_insert() {
		let scorer = SqlScorer::generic();
		let output = serde_json::json!("INSERT INTO users (name, age) VALUES ('John', 30)");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}

	#[test]
	fn test_sql_invalid() {
		let scorer = SqlScorer::generic();
		let output = serde_json::json!("SELECT * FROM WHERE");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).unwrap();
		assert!(!score.passed);
		assert_eq!(score.score, 0.0);
	}

	#[test]
	fn test_sql_from_json_object() {
		let scorer = SqlScorer::generic();
		let output = serde_json::json!({
			"sql": "SELECT id, name FROM products"
		});
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}

	#[test]
	fn test_sql_postgres_specific() {
		let scorer = SqlScorer::postgres();
		let output = serde_json::json!("SELECT * FROM users LIMIT 10 OFFSET 20");
		let expected = serde_json::json!("");
		let score = scorer.score(&expected, &output).unwrap();
		assert!(score.passed);
		assert_eq!(score.score, 1.0);
	}
}

