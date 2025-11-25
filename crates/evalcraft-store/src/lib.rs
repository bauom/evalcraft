use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

// Use shared types
use evalcraft_types::EvalResult;

#[derive(Debug)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEntity {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl Store {
    /// Open a new store at the given path (e.g., "eval.db")
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI,
        )?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        store.init_schema()?;
        Ok(store)
    }

    /// Initialize the SQLite schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS runs (
                id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL,
                metadata TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS evals (
                id INTEGER PRIMARY KEY,
                run_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                summary TEXT,
                FOREIGN KEY(run_id) REFERENCES runs(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS results (
                id INTEGER PRIMARY KEY,
                eval_id INTEGER NOT NULL,
                case_id TEXT,
                input TEXT NOT NULL,
                output TEXT NOT NULL,
                expected TEXT NOT NULL,
                error TEXT,
                FOREIGN KEY(eval_id) REFERENCES evals(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS scores (
                id INTEGER PRIMARY KEY,
                result_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                value REAL NOT NULL,
                passed BOOLEAN NOT NULL,
                details TEXT,
                FOREIGN KEY(result_id) REFERENCES results(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS traces (
                id INTEGER PRIMARY KEY,
                result_id INTEGER NOT NULL,
                model TEXT,
                duration_ms INTEGER,
                input TEXT,
                output TEXT,
                tokens_in INTEGER,
                tokens_out INTEGER,
                FOREIGN KEY(result_id) REFERENCES results(id)
            )",
            [],
        )?;

        Ok(())
    }

    /// Create a new run entry
    pub fn create_run(&self, metadata: Option<serde_json::Value>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        
        conn.execute(
            "INSERT INTO runs (created_at, metadata) VALUES (?1, ?2)",
            params![now.to_rfc3339(), metadata.map(|v| v.to_string())],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    /// Save a full evaluation result into the database
    pub fn save_eval(&self, run_id: i64, name: &str, result: &EvalResult) -> Result<i64> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // 1. Create Eval
        tx.execute(
            "INSERT INTO evals (run_id, name, summary) VALUES (?1, ?2, ?3)",
            params![
                run_id, 
                name, 
                serde_json::to_string(&result.summary).ok()
            ],
        )?;
        let eval_id = tx.last_insert_rowid();

        // 2. Save Results
        for case in &result.cases {
            tx.execute(
                "INSERT INTO results (eval_id, case_id, input, output, expected, error) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    eval_id,
                    case.case.id,
                    case.case.input.to_string(),
                    case.output.to_string(),
                    case.case.expected.to_string(),
                    case.error
                ],
            )?;
            let result_id = tx.last_insert_rowid();

            // 3. Save Scores
            for score in &case.scores {
                tx.execute(
                    "INSERT INTO scores (result_id, name, value, passed, details) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        result_id,
                        score.name,
                        score.value,
                        score.passed,
                        score.details.as_ref().map(|d| d.to_string())
                    ],
                )?;
            }

            // 4. Save Traces
            for trace in &case.traces {
                tx.execute(
                    "INSERT INTO traces (result_id, model, duration_ms, input, output, tokens_in, tokens_out) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        result_id,
                        trace.model,
                        trace.duration_ms,
                        trace.input.to_string(),
                        trace.output.to_string(),
                        trace.usage.as_ref().map(|u| u.input_tokens),
                        trace.usage.as_ref().map(|u| u.output_tokens),
                    ],
                )?;
            }
        }

        tx.commit()?;
        Ok(eval_id)
    }
}
