use std::error::Error;
use dotenvy::dotenv;
use std::time::{Duration, Instant};
use tracing::{warn, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use reqwest;
use once_cell::sync::Lazy;

// Type alias for a thread-safe error type
type BoxedError = Box<dyn Error + Send + Sync>;

// Cache structure to avoid redundant API checks
struct ApiStatusCache {
    last_check_time: Instant,
    last_status: bool,
    cache_duration: Duration,
}

// Track the last time we logged an API status message to avoid duplicate logs
static LAST_LOG_TIME: Lazy<Mutex<Instant>> = Lazy::new(|| {
    Mutex::new(Instant::now() - Duration::from_secs(3600)) // Start with expired timestamp
});

// Minimum time between log messages (in seconds)
const LOG_THROTTLE_DURATION: u64 = 5;

// Global API status cache with a default 10-second cache duration
static API_STATUS_CACHE: Lazy<Mutex<ApiStatusCache>> = Lazy::new(|| {
    Mutex::new(ApiStatusCache {
        last_check_time: Instant::now() - Duration::from_secs(3600), // Start with expired cache
        last_status: true, // Assume available initially
        cache_duration: Duration::from_secs(10), // Cache for 10 seconds
    })
});

// Global API status monitor to ensure we only start one
static API_STATUS_MONITOR: Lazy<Arc<AtomicBool>> = Lazy::new(|| {
    // Start the monitor with a 30-second check interval
    start_api_status_monitor_internal(Duration::from_secs(30))
});

/// Check API availability using a lightweight GET request to the Anthropic API
/// This doesn't consume any tokens as it doesn't actually run inference
pub async fn check_api_availability_lightweight() -> Result<bool, BoxedError> {
    // First check the global monitor status
    let monitor_status = API_STATUS_MONITOR.load(Ordering::SeqCst);

    // If the monitor indicates API is available, check the cache first
    // Otherwise, proceed with a fresh check
    if monitor_status {
        // Check if we have a recent cached result
        {
            let cache = API_STATUS_CACHE.lock().unwrap();
            if cache.last_check_time.elapsed() < cache.cache_duration {
                return Ok(cache.last_status);
            }
        }
    }

    // No recent cache, perform the actual check
    dotenv().ok();

    // Get API key from environment
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => return Err("ANTHROPIC_API_KEY environment variable not set".into()),
    };

    // Create a client with a short timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // Make a GET request to the models endpoint
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    let status = match response {
        Ok(res) => {
            let status = res.status();
            if status.is_success() || status.as_u16() == 401 { // 401 means auth failed but API is up
                // Throttle logging to avoid duplicate messages
                let should_log = {
                    let mut last_log = LAST_LOG_TIME.lock().unwrap();
                    let should_log = last_log.elapsed() > Duration::from_secs(LOG_THROTTLE_DURATION);
                    if should_log {
                        *last_log = Instant::now();
                    }
                    should_log
                };

                if should_log {
                    info!("Anthropic API is available (status: {})", status);
                }
                true
            } else if status.as_u16() == 429 || status.as_u16() == 503 {
                warn!("Anthropic API is overloaded (status: {})", status);
                false
            } else {
                warn!("Unexpected status from Anthropic API: {}", status);
                false // Assume unavailable for unexpected status
            }
        },
        Err(e) => {
            warn!("Error checking Anthropic API: {}", e);
            if e.is_timeout() || e.is_connect() {
                false // API is likely overloaded or down
            } else {
                return Err(Box::new(e));
            }
        }
    };

    // Update the cache with the new result
    let mut cache = API_STATUS_CACHE.lock().unwrap();
    cache.last_check_time = Instant::now();
    cache.last_status = status;

    Ok(status)
}

/// Verifies that the Anthropic API key is valid and the API is available
/// This is a lightweight check that doesn't consume any tokens
pub async fn verify_anthropic_api_key() -> Result<(), BoxedError> {
    match check_api_availability_lightweight().await {
        Ok(true) => Ok(()),
        Ok(false) => Err("Anthropic API is currently unavailable or overloaded".into()),
        Err(e) => Err(e),
    }
}

/// Returns a handle to the global API status monitor
/// This ensures we only have one monitor running in the application
pub fn start_api_status_monitor(_check_interval: Duration) -> Arc<AtomicBool> {
    // Return the global monitor instead of creating a new one
    API_STATUS_MONITOR.clone()
}

// Internal function to start the actual monitor
fn start_api_status_monitor_internal(check_interval: Duration) -> Arc<AtomicBool> {
    let api_available = Arc::new(AtomicBool::new(true)); // Assume available initially
    let api_status_clone = api_available.clone();

    tokio::spawn(async move {
        loop {
            // Use the lightweight check that doesn't consume tokens
            let available = (check_api_availability_lightweight().await).unwrap_or(false);

            api_status_clone.store(available, Ordering::SeqCst);

            // Adjust sleep time based on API status
            let sleep_duration = if available {
                check_interval
            } else {
                Duration::from_secs(5) // Check more frequently when API is down
            };

            tokio::time::sleep(sleep_duration).await;
        }
    });

    api_available
}

/// Waits until the Anthropic API is available
/// Useful before starting operations that require the API
pub async fn wait_for_api_availability(timeout: Option<Duration>) -> bool {
    let start_time = std::time::Instant::now();
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        // Check if we've exceeded the timeout
        if let Some(timeout_duration) = timeout {
            if start_time.elapsed() > timeout_duration {
                warn!("Timeout waiting for Anthropic API availability");
                return false;
            }
        }

        // First check the global monitor status to avoid unnecessary API calls
        let monitor_status = API_STATUS_MONITOR.load(Ordering::SeqCst);
        if monitor_status {
            info!("Proceeding with operation");
            return true;
        }

        // If monitor says API is down, do a direct check to confirm
        let available = match check_api_availability_lightweight().await {
            Ok(status) => status,
            Err(e) => {
                warn!("Error checking API availability: {}", e);
                false // Assume unavailable on error
            }
        };

        if available {
            info!("Proceeding with operation");
            return true;
        } else {
            warn!("Anthropic API is overloaded, waiting {:?} before checking again", backoff);
            tokio::time::sleep(backoff).await;

            // Increase backoff with a cap
            backoff = std::cmp::min(Duration::from_millis((backoff.as_millis() as f32 * 1.5) as u64), max_backoff);
        }
    }
}

/// Check the Anthropic API status by using the cached monitor status
/// This is a lightweight check that doesn't make an actual API call if the monitor
/// has recently verified the API status
pub async fn check_anthropic_api_status() -> Result<bool, BoxedError> {
    // First check the global monitor status
    let monitor_status = API_STATUS_MONITOR.load(Ordering::SeqCst);
    if monitor_status {
        // If the monitor says the API is available, return true
        return Ok(true);
    }

    // If the monitor says the API is not available, do a direct check
    // to get the most up-to-date status
    check_api_availability_lightweight().await
}
