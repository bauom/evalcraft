use crate::types::EvalResult;
use anyhow::Result;

/// Helper to assert evaluation pass rate meets a threshold.
///
/// Use this in your `#[tokio::test]` functions.
///
/// # Example
/// ```ignore
/// #[tokio::test]
/// async fn test_my_agent() -> Result<()> {
///     let eval = Eval::builder()
///         .data_source(data)
///         .task(task)
///         .scorers(scorers)
///         .build()?;
///     
///     let result = eval.run().await?;
///     
///     // Assert 80% pass rate
///     assert_eval_pass_rate(&result, 0.8)?;
///     
///     Ok(())
/// }
/// ```
pub fn assert_eval_pass_rate(result: &EvalResult, min_pass_rate: f64) -> Result<()> {
    if result.summary.pass_rate < min_pass_rate {
        anyhow::bail!(
            "Evaluation failed: pass rate {:.1}% is below threshold {:.1}%\n{}",
            result.summary.pass_rate * 100.0,
            min_pass_rate * 100.0,
            result.summary_table()
        );
    }
    Ok(())
}

/// Helper to assert average score meets a threshold.
pub fn assert_eval_avg_score(result: &EvalResult, min_avg_score: f64) -> Result<()> {
    if result.summary.avg_score < min_avg_score {
        anyhow::bail!(
            "Evaluation failed: avg score {:.3} is below threshold {:.3}\n{}",
            result.summary.avg_score,
            min_avg_score,
            result.summary_table()
        );
    }
    Ok(())
}

/// Helper to assert all cases passed.
pub fn assert_eval_all_passed(result: &EvalResult) -> Result<()> {
    if result.summary.passed != result.summary.total {
        anyhow::bail!(
            "Evaluation failed: {}/{} cases passed\n{}",
            result.summary.passed,
            result.summary.total,
            result.summary_table()
        );
    }
    Ok(())
}


