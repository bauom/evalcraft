use crate::types::EvalResult;

pub fn generate_html_report(result: &EvalResult) -> String {
    let mut rows = String::new();
    
    for cr in &result.cases {
        let id = cr.case.id.as_deref().unwrap_or("-");
        let all_passed = !cr.scores.is_empty() && cr.scores.iter().all(|s| s.passed);
        let passed_icon = if all_passed { "✓" } else { "✗" };
        let row_class = if all_passed { "pass" } else { "fail" };
        
        let avg = if cr.scores.is_empty() {
            0.0
        } else {
            let sum: f64 = cr.scores.iter().map(|s| s.value).sum();
            sum / (cr.scores.len() as f64)
        };
        
        let input_str = serde_json::to_string_pretty(&cr.case.input).unwrap_or_default();
        let output_str = serde_json::to_string_pretty(&cr.output).unwrap_or_default();
        let expected_str = serde_json::to_string_pretty(&cr.case.expected).unwrap_or_default();
        
        let mut scores_html = String::new();
        for score in &cr.scores {
            let score_class = if score.passed { "pass" } else { "fail" };
            scores_html.push_str(&format!(
                r#"<span class="badge {}">{}: {:.3}</span>"#,
                score_class, score.name, score.value
            ));
        }
        
        // Build traces HTML
        let traces_html = if !cr.traces.is_empty() {
            let mut traces_content = String::new();
            for (i, trace) in cr.traces.iter().enumerate() {
                let model = trace.model.as_deref().unwrap_or("unknown");
                let duration = trace.duration_ms.map(|d| format!("{}ms", d)).unwrap_or_else(|| "-".to_string());
                
                let usage_str = if let Some(usage) = &trace.usage {
                    format!("🪙 {} in / {} out / {} total", usage.input_tokens, usage.output_tokens, usage.total_tokens)
                } else {
                    "".to_string()
                };
                
                let trace_input = serde_json::to_string_pretty(&trace.input).unwrap_or_default();
                let trace_output = serde_json::to_string_pretty(&trace.output).unwrap_or_default();
                
                traces_content.push_str(&format!(
                    r#"
                    <div class="trace-item">
                        <div class="trace-header">
                            <strong>Trace #{}</strong>
                            <span class="trace-meta">🤖 {} • ⏱️ {}</span>
                        </div>
                        <div class="trace-usage">{}</div>
                        <div class="trace-details">
                            <div class="trace-section">
                                <strong>Input:</strong>
                                <pre>{}</pre>
                            </div>
                            <div class="trace-section">
                                <strong>Output:</strong>
                                <pre>{}</pre>
                            </div>
                        </div>
                    </div>
                    "#,
                    i + 1, model, duration, usage_str,
                    html_escape(&trace_input),
                    html_escape(&trace_output)
                ));
            }
            
            format!(
                r#"<button class="trace-toggle" onclick="toggleTraces('trace-{}')">{} trace(s)</button>
                <div id="trace-{}" class="trace-container" style="display: none;">{}</div>"#,
                id, cr.traces.len(), id, traces_content
            )
        } else {
            String::new()
        };
        
        rows.push_str(&format!(
            r#"
            <tr class="{}">
                <td>{}</td>
                <td class="icon">{}</td>
                <td class="avg-score">{:.3}</td>
                <td><pre>{}</pre></td>
                <td><pre>{}</pre></td>
                <td><pre>{}</pre></td>
                <td class="scores">{}</td>
            </tr>
            <tr class="trace-row {}">
                <td colspan="7">{}</td>
            </tr>
            "#,
            row_class, id, passed_icon, avg, 
            html_escape(&input_str), 
            html_escape(&output_str), 
            html_escape(&expected_str),
            scores_html,
            row_class,
            traces_html
        ));
    }
    
    let pass_rate_class = if result.summary.pass_rate >= 0.8 { "good" } else if result.summary.pass_rate >= 0.5 { "warn" } else { "bad" };
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Evalcraft Report</title>
    <style>
        * {{ box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        h1 {{
            margin: 0 0 10px 0;
            color: #333;
        }}
        .summary {{
            display: flex;
            gap: 20px;
            margin: 20px 0 30px 0;
            padding: 20px;
            background: #f8f9fa;
            border-radius: 6px;
        }}
        .summary-item {{
            flex: 1;
        }}
        .summary-label {{
            font-size: 12px;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 5px;
        }}
        .summary-value {{
            font-size: 28px;
            font-weight: 600;
            color: #333;
        }}
        .summary-value.good {{ color: #28a745; }}
        .summary-value.warn {{ color: #ffc107; }}
        .summary-value.bad {{ color: #dc3545; }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }}
        th {{
            background: #343a40;
            color: white;
            padding: 12px;
            text-align: left;
            font-weight: 600;
            font-size: 13px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}
        td {{
            padding: 12px;
            border-bottom: 1px solid #dee2e6;
            vertical-align: top;
        }}
        tr.pass {{ background: #f0f9f4; }}
        tr.fail {{ background: #fef3f2; }}
        tr:hover {{ background: #e9ecef; }}
        .icon {{
            text-align: center;
            font-size: 18px;
            width: 50px;
        }}
        .avg-score {{
            font-weight: 600;
            color: #495057;
        }}
        pre {{
            margin: 0;
            padding: 8px;
            background: #f8f9fa;
            border-radius: 4px;
            font-size: 12px;
            max-height: 150px;
            overflow: auto;
            white-space: pre-wrap;
            word-break: break-word;
        }}
        .scores {{
            display: flex;
            flex-wrap: wrap;
            gap: 6px;
        }}
        .badge {{
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 600;
            white-space: nowrap;
        }}
        .badge.pass {{
            background: #d4edda;
            color: #155724;
        }}
        .badge.fail {{
            background: #f8d7da;
            color: #721c24;
        }}
        .timestamp {{
            color: #6c757d;
            font-size: 14px;
            margin-bottom: 20px;
        }}
        .trace-row {{
            border-bottom: none !important;
        }}
        .trace-row td {{
            padding: 0 12px 12px 12px !important;
            border-bottom: 1px solid #dee2e6;
        }}
        .trace-toggle {{
            background: #007bff;
            color: white;
            border: none;
            padding: 6px 12px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
            font-weight: 600;
            transition: background 0.2s;
        }}
        .trace-toggle:hover {{
            background: #0056b3;
        }}
        .trace-container {{
            margin-top: 12px;
            padding: 12px;
            background: #f8f9fa;
            border-radius: 6px;
            border-left: 3px solid #007bff;
        }}
        .trace-item {{
            margin-bottom: 16px;
            padding: 12px;
            background: white;
            border-radius: 4px;
            border: 1px solid #e9ecef;
        }}
        .trace-item:last-child {{
            margin-bottom: 0;
        }}
        .trace-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 8px;
            padding-bottom: 8px;
            border-bottom: 1px solid #e9ecef;
        }}
        .trace-meta {{
            font-size: 12px;
            color: #6c757d;
        }}
        .trace-usage {{
            font-size: 12px;
            color: #495057;
            margin-bottom: 12px;
            font-weight: 500;
        }}
        .trace-details {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 12px;
        }}
        .trace-section {{
            background: #f8f9fa;
            padding: 8px;
            border-radius: 4px;
        }}
        .trace-section strong {{
            display: block;
            margin-bottom: 6px;
            font-size: 11px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: #6c757d;
        }}
        .trace-section pre {{
            margin: 0;
            max-height: 200px;
            font-size: 11px;
        }}
    </style>
    <script>
        function toggleTraces(id) {{
            const element = document.getElementById(id);
            if (element.style.display === 'none') {{
                element.style.display = 'block';
            }} else {{
                element.style.display = 'none';
            }}
        }}
    </script>
</head>
<body>
    <div class="container">
        <h1>🧪 Evalcraft Report</h1>
        <div class="timestamp">Generated: {}</div>
        
        <div class="summary">
            <div class="summary-item">
                <div class="summary-label">Total Cases</div>
                <div class="summary-value">{}</div>
            </div>
            <div class="summary-item">
                <div class="summary-label">Passed</div>
                <div class="summary-value good">{}</div>
            </div>
            <div class="summary-item">
                <div class="summary-label">Failed</div>
                <div class="summary-value bad">{}</div>
            </div>
            <div class="summary-item">
                <div class="summary-label">Pass Rate</div>
                <div class="summary-value {}">{:.1}%</div>
            </div>
            <div class="summary-item">
                <div class="summary-label">Avg Score</div>
                <div class="summary-value">{:.3}</div>
            </div>
        </div>
        
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Status</th>
                    <th>Avg Score</th>
                    <th>Input</th>
                    <th>Output</th>
                    <th>Expected</th>
                    <th>Scores</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
</body>
</html>"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        result.summary.total,
        result.summary.passed,
        result.summary.total - result.summary.passed,
        pass_rate_class,
        result.summary.pass_rate * 100.0,
        result.summary.avg_score,
        rows
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}


