use book_generator::utils::api_verification::{check_anthropic_api_status, check_api_availability_lightweight, wait_for_api_availability};
use std::time::Duration;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut lightweight = false;
    let mut wait_mode = false;
    let mut timeout_secs = 60;
    
    for arg in &args {
        if arg == "--lightweight" || arg == "-l" {
            lightweight = true;
        } else if arg == "--wait" || arg == "-w" {
            wait_mode = true;
        } else if arg.starts_with("--timeout=") {
            if let Some(value) = arg.strip_prefix("--timeout=") {
                if let Ok(secs) = value.parse::<u64>() {
                    timeout_secs = secs;
                }
            }
        }
    }
    
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("Usage: check_api [OPTIONS]");
        println!("Options:");
        println!("  --lightweight, -l    Use lightweight check that doesn't consume tokens");
        println!("  --wait, -w           Wait for API to become available");
        println!("  --timeout=SECONDS    Maximum time to wait (default: 60 seconds)");
        println!("  --help, -h           Show this help message");
        return Ok(());
    }
    
    if wait_mode {
        println!("Waiting for Anthropic API to become available (timeout: {}s)...", timeout_secs);
        let available = wait_for_api_availability(Some(Duration::from_secs(timeout_secs))).await;
        if available {
            println!("✅ Anthropic API is now available and ready for use");
        } else {
            println!("⏱️ Timeout reached. API is still not available.");
        }
        return Ok(());
    }
    
    println!("Checking Anthropic API status...");
    
    if lightweight {
        println!("Using lightweight check (no token usage)");
        match check_api_availability_lightweight().await {
            Ok(true) => println!("✅ Anthropic API is available (lightweight check)"),
            Ok(false) => println!("⚠️ Anthropic API is currently overloaded (lightweight check)"),
            Err(e) => println!("❌ Error checking Anthropic API: {}", e),
        }
    } else {
        println!("Using standard check");
        match check_anthropic_api_status().await {
            Ok(true) => println!("✅ Anthropic API is available and ready for use"),
            Ok(false) => println!("⚠️ Anthropic API is currently overloaded. You may experience delays."),
            Err(e) => println!("❌ Error checking Anthropic API: {}", e),
        }
    }
    
    Ok(())
} 