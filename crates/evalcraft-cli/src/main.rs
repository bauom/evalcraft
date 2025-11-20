use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use evalcraft_core::{
	from_async_fn, Eval, ExactMatchScorer, JsonlDataSource, LevenshteinScorer, Scorer,
	ContainsScorer, RegexScorer, JsonScorer, SqlScorer, SqlDialect,
};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(name = "evalcraft", about = "Run agent and LLM evaluations")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
	Run(RunArgs),
}

#[derive(Debug, Clone, Parser)]
struct RunArgs {
	/// JSONL file containing lines with fields: { "id"?: string, "input": any, "expected": any }
	#[arg(long)]
	data: PathBuf,

	/// Concurrency (cases in-flight)
	#[arg(long, default_value_t = 8)]
	concurrency: usize,

	/// Use exact-match scorer
	#[arg(long, action = ArgAction::SetTrue)]
	exact: bool,

	/// Use Levenshtein scorer with given min similarity (0.0..=1.0)
	#[arg(long)]
	levenshtein: Option<f64>,

	/// Check if output contains substring (case-sensitive)
	#[arg(long)]
	contains: Option<String>,

	/// Check if output contains substring (case-insensitive)
	#[arg(long)]
	contains_i: Option<String>,

	/// Validate output matches regex pattern
	#[arg(long)]
	regex: Option<String>,

	/// Validate output is valid JSON
	#[arg(long, action = ArgAction::SetTrue)]
	json: bool,

	/// Validate output JSON against a schema file
	#[arg(long)]
	json_schema: Option<PathBuf>,

	/// Validate output is valid SQL (generic dialect)
	#[arg(long, action = ArgAction::SetTrue)]
	sql: bool,

	/// SQL dialect: generic, postgres, mysql, sqlite
	#[arg(long, default_value = "generic")]
	sql_dialect: String,

	/// Output JSON result to a file
	#[arg(long)]
	json_out: Option<PathBuf>,

	/// HTTP task endpoint (POST by default). Sends { "input": <value> } and expects JSON response.
	#[arg(long)]
	http_url: Option<String>,

	/// HTTP method for --http-url (GET or POST)
	#[arg(long, default_value = "POST")]
	http_method: String,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Run(args) => run(args).await?,
	}
	Ok(())
}

async fn run(args: RunArgs) -> Result<()> {
	let data = Arc::new(JsonlDataSource::new(&args.data));

	let task = if let Some(url) = args.http_url {
		let method = args.http_method.to_uppercase();
		from_async_fn(move |input| {
			let url = url.clone();
			let method = method.clone();
			let input = input.clone();
			async move {
				let client = reqwest::Client::new();
				let resp = match method.as_str() {
					"GET" => {
						// Encode input as query ?input=<json>
						let q = [("input", input.to_string())];
						client.get(&url).query(&q).send().await?
					}
					_ => {
						client.post(&url).json(&json!({ "input": input })).send().await?
					}
				};
				let status = resp.status();
				let v = resp.json::<serde_json::Value>().await?;
				if !status.is_success() {
					anyhow::bail!("HTTP {}: {}", status.as_u16(), v);
				}
				Ok(v)
			}
		})
	} else {
		// Default "echo" task: append " World!" to string inputs
		from_async_fn(|input| {
			let input = input.clone();
			async move {
				let s = input.as_str().unwrap_or_default();
				Ok(json!(format!("{s} World!")))
			}
		})
	};

	let mut scorers: Vec<Arc<dyn Scorer>> = Vec::new();
	
	// Add exact match scorer
	if args.exact {
		scorers.push(Arc::new(ExactMatchScorer));
	}
	
	// Add Levenshtein scorer
	if let Some(min_sim) = args.levenshtein {
		scorers.push(Arc::new(LevenshteinScorer::new(min_sim)));
	}
	
	// Add contains scorers
	if let Some(substring) = args.contains {
		scorers.push(Arc::new(ContainsScorer::new(substring)));
	}
	if let Some(substring) = args.contains_i {
		scorers.push(Arc::new(ContainsScorer::case_insensitive(substring)));
	}
	
	// Add regex scorer
	if let Some(pattern) = args.regex {
		let regex_scorer = RegexScorer::new(&pattern)?;
		scorers.push(Arc::new(regex_scorer));
	}
	
	// Add JSON scorers
	if args.json {
		scorers.push(Arc::new(JsonScorer::new()));
	}
	if let Some(schema_path) = args.json_schema {
		let schema_content = tokio::fs::read_to_string(&schema_path).await?;
		let schema: serde_json::Value = serde_json::from_str(&schema_content)?;
		let json_scorer = JsonScorer::with_schema(schema)?;
		scorers.push(Arc::new(json_scorer));
	}
	
	// Add SQL scorer
	if args.sql {
		let dialect = match args.sql_dialect.to_lowercase().as_str() {
			"postgres" | "postgresql" => SqlDialect::PostgreSQL,
			"mysql" => SqlDialect::MySQL,
			"sqlite" => SqlDialect::SQLite,
			_ => SqlDialect::Generic,
		};
		scorers.push(Arc::new(SqlScorer::new(dialect)));
	}
	
	// Default to exact if no scorers specified
	if scorers.is_empty() {
		scorers.push(Arc::new(ExactMatchScorer));
	}

	let eval = Eval::builder()
		.data_source(data)
		.task(task)
		.scorers(scorers)
		.concurrency(args.concurrency)
		.build()?;

	let result = eval.run().await?;
	println!("{}", result.summary_table());

	if let Some(path) = args.json_out {
		let json = serde_json::to_string_pretty(&result)?;
		tokio::fs::write(path, json).await?;
	}

	Ok(())
}


