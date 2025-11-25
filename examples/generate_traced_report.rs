// Quick script to generate HTML report from the traced results

use std::fs;
use evalcraft_core::{EvalResult, generate_html_report};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read the traced results JSON
    let json_content = fs::read_to_string("traced_results.json")?;
    let result: EvalResult = serde_json::from_str(&json_content)?;
    
    // Generate HTML report
    let html = generate_html_report(&result);
    
    // Save it
    fs::write("traced_report.html", html)?;
    
    println!("✓ HTML report with traces saved to: traced_report.html");
    println!("  Open it in your browser to see interactive traces!");
    
    Ok(())
}




