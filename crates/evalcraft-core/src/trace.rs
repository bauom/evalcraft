pub use evalcraft_types::{Trace, TokenUsage, TraceBuilder};

// Thread-local storage for collecting traces during test execution
use std::cell::RefCell;

tokio::task_local! {
    static TRACES: RefCell<Vec<Trace>>;
}

/// Run a future within a tracing scope and return the result along with collected traces.
pub async fn scope_traces<F, R>(f: F) -> (R, Vec<Trace>)
where
    F: std::future::Future<Output = R>,
{
    let traces = RefCell::new(Vec::new());
    TRACES.scope(traces, async move {
        let result = f.await;
        let collected = TRACES.with(|t| t.borrow().clone());
        (result, collected)
    }).await
}

/// Report a trace (adds it to the current task's trace collection)
pub fn report_trace(trace: Trace) {
    let _ = TRACES.try_with(|traces| {
        traces.borrow_mut().push(trace);
    });
}

/// Get all traces for the current task
pub fn get_traces() -> Vec<Trace> {
    TRACES.try_with(|traces| traces.borrow().clone()).unwrap_or_default()
}

/// Clear traces (called automatically between test cases)
pub fn clear_traces() {
    let _ = TRACES.try_with(|traces| {
        traces.borrow_mut().clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_trace_builder() {
        let trace = Trace::start_now()
            .model("gpt-4o-mini")
            .id("trace-1")
            .finish(
                json!({"messages": [{"role": "user", "content": "Hello"}]}),
                json!({"content": "Hi there!"}),
                Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                }),
            );
        
        assert_eq!(trace.model, Some("gpt-4o-mini".to_string()));
        assert!(trace.duration_ms.is_some());
        assert!(trace.error.is_none());
    }
    
    #[tokio::test]
    async fn test_report_and_get_traces() {
        // We need to run this inside the trace scope
        let (_, traces) = scope_traces(async {
            clear_traces();
            
            let trace1 = Trace::start_now()
                .model("gpt-4")
                .finish(json!("input1"), json!("output1"), None);
            
            let trace2 = Trace::start_now()
                .model("claude-3")
                .finish(json!("input2"), json!("output2"), None);
            
            report_trace(trace1);
            report_trace(trace2);
            
            let traces = get_traces();
            assert_eq!(traces.len(), 2);
            
            clear_traces();
            assert_eq!(get_traces().len(), 0);
        }).await;
        
        // After scope ends, we can inspect the returned traces if we wanted
        // but the assertions inside are what we care about here.
        // If we didn't use scope_traces, report_trace would be a no-op/fail silently
    }
}
