use anyhow::{Context, Result};
use console::style;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use ipnet::Ipv4Net;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use std::net::Ipv4Addr;
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::time::Instant;
use serde::Deserialize;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::{
    cursor,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::io::Write;  // Add this import

// Repository Information
const REPO_URL: &str = "github.com/zonay/public-ollama-finder";

// Add styling constants
const HEADER_STYLE: &str = "╭─ 🌐 ";
const LIST_ITEM_STYLE: &str = "├─ ";
const LAST_ITEM_STYLE: &str = "╰─ ";

static STOP_SCAN: AtomicBool = AtomicBool::new(false);
static PAUSE_SCAN: AtomicBool = AtomicBool::new(false);
// Reduce concurrent connections to be more CPU friendly
const CONCURRENT_LIMIT: usize = 2000;
const RATE_LIMIT_PER_SECOND: u32 = 800;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ScanResult {
    ip: String,
    status: u16,
    location: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelDetails {
    parent_model: String,
    format: String,
    family: String,
    parameter_size: String,
    quantization_level: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Model {
    name: String,
    model: String,
    modified_at: String,
    size: u64,
    digest: String,
    details: ModelDetails,
}

#[derive(Debug, Clone, Deserialize)]
struct TagsResponse {
    models: Vec<Model>,
}

fn console_log(msg: String) {
    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::MoveToColumn(0));
    print!("{}\n", msg);
    let _ = stdout.flush();
}

async fn check_host(
    ip: String,
    location: String,
    client: &reqwest::Client,
    semaphore: Arc<Semaphore>,
    model_writer: Arc<tokio::sync::Mutex<csv::Writer<std::fs::File>>>,
    endpoint_writer: Arc<tokio::sync::Mutex<csv::Writer<std::fs::File>>>,
) -> Option<ScanResult> {
    if STOP_SCAN.load(Ordering::Relaxed) {
        return None;
    }

    let _permit = semaphore.acquire().await.ok()?;
    let url = format!("http://{}:11434/api/tags", ip);

    match client.get(&url).timeout(Duration::from_millis(500)).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            match status {
                200 => {
                    if let Ok(tags_response) = response.json::<TagsResponse>().await {
                        let mut model_writer = model_writer.lock().await;
                        
                        // Enhanced server info display
                        console_log(format!("\n{}{}", 
                            HEADER_STYLE,
                            style("Found Ollama Server").green().bold()
                        ));
                        console_log(format!("{}API Endpoint: {}", 
                            LIST_ITEM_STYLE,
                            style(&url).cyan()
                        ));
                        console_log(format!("{}Server URL: {}", 
                            LIST_ITEM_STYLE,
                            style(format!("http://{}:11434", ip)).cyan()
                        ));

                        // Enhanced model list display
                        if !tags_response.models.is_empty() {
                            let mut models: Vec<_> = tags_response.models
                                .iter()
                                .map(|m| {
                                    let size_gb = m.size as f64 / 1_073_741_824.0;
                                    (m.name.as_str(), size_gb)
                                })
                                .collect();
                            models.sort_by(|a, b| a.0.cmp(b.0));
                            
                            console_log(format!("{}Available Models:", LIST_ITEM_STYLE));
                            for (i, (name, size)) in models.iter().enumerate() {
                                let is_last = i == models.len() - 1;
                                let prefix = if is_last { LAST_ITEM_STYLE } else { LIST_ITEM_STYLE };
                                let size_str = if *size > 0.0 {
                                    style(format!(" ({:.2} GB)", size)).dim().to_string()
                                } else {
                                    "".to_string()
                                };
                                console_log(format!("{}{}{}{}",
                                    "  ",  // Indent for nested items
                                    prefix,
                                    style(format!("{}. {}", i + 1, name)).blue(),
                                    size_str
                                ));
                            }
                            console_log("".to_string());
                        }
                        
                        for model in tags_response.models {
                            let size_gb = model.size as f64 / 1_073_741_824.0;
                            model_writer.write_record(&[
                                &format!("http://{}:11434", ip),
                                &model.name,
                                &model.model,
                                &model.modified_at,
                                &format!("{:.2}", size_gb), // Format size to 2 decimal places
                                &model.digest,
                                &model.details.parent_model,
                                &model.details.format,
                                &model.details.family,
                                &model.details.parameter_size,
                                &model.details.quantization_level,
                            ]).unwrap();
                            model_writer.flush().unwrap();
                        }
                    }
                    let mut endpoint_writer = endpoint_writer.lock().await;
                    endpoint_writer.write_record(&[
                        &format!("http://{}:11434", ip),
                        &url,
                        &status.to_string(),
                        &location,
                    ]).unwrap();
                    endpoint_writer.flush().unwrap();
                    Some(ScanResult {
                        ip,
                        status,
                        location,
                    })
                }
                404 => {
                    console_log(format!("{}{}",
                        LIST_ITEM_STYLE,
                        style(format!("Possible Ollama server (404): {}", url)).yellow()
                    ));
                    None
                }
                _ => None,
            }
        }
        Err(_) => None,
    }
}

fn parse_ip_range(input: &str) -> Result<Ipv4Net> {
    // Try CIDR format first (e.g., "192.168.1.0/24")
    if let Ok(network) = input.parse::<Ipv4Net>() {
        return Ok(network);
    }

    // Try range format (e.g., "192.168.1.1-192.168.1.255")
    if input.contains('-') {
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 2 {
            let start: Ipv4Addr = parts[0].trim().parse()?;
            let end: Ipv4Addr = parts[1].trim().parse()?;
            
            // Convert range to CIDR blocks
            let start_u32: u32 = start.into();
            let end_u32: u32 = end.into();
            
            // Find the largest matching CIDR block
            let prefix_len = 32 - (end_u32 - start_u32 + 1).trailing_zeros();
            let network = Ipv4Net::new(start, prefix_len as u8)?;
            return Ok(network);
        }
    }

    // Try single IP (convert to /32 CIDR)
    if let Ok(ip) = input.parse::<Ipv4Addr>() {
        return Ok(Ipv4Net::new(ip, 32)?);
    }

    anyhow::bail!("Invalid IP range format: {}", input)
}

fn extract_ip_ranges(text: &str) -> Vec<(String, String)> {
    let mut ranges = Vec::new();
    
    // Updated regex patterns to be compatible with Rust's regex engine
    let cidr_pattern = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/\d{1,2})").unwrap();
    let range_pattern = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\s*-\s*(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})").unwrap();
    let single_ip_pattern = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})(?:[^/\d]|$)").unwrap();
    
    // Try parsing as JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        fn extract_from_value(value: &serde_json::Value) -> Vec<String> {
            match value {
                serde_json::Value::String(s) => vec![s.clone()],
                serde_json::Value::Array(arr) => arr.iter()
                    .flat_map(extract_from_value)
                    .collect(),
                serde_json::Value::Object(obj) => obj.values()
                    .flat_map(extract_from_value)
                    .collect(),
                _ => vec![],
            }
        }
        
        for ip_text in extract_from_value(&json) {
            ranges.push((ip_text, "JSON".to_string()));
        }
        return ranges;
    }

    // Process line by line for other formats
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Try CIDR notation
        if let Some(cap) = cidr_pattern.captures(line) {
            ranges.push((cap[1].to_string(), "CIDR".to_string()));
            continue;
        }

        // Try IP range format
        if let Some(cap) = range_pattern.captures(line) {
            ranges.push((format!("{}-{}", &cap[1], &cap[2]), "Range".to_string()));
            continue;
        }

        // Try single IP
        if let Some(cap) = single_ip_pattern.captures(line) {
            ranges.push((format!("{}/32", &cap[1]), "Single IP".to_string()));
        }
    }

    ranges
}

fn load_ranges() -> Result<Vec<(Ipv4Net, String)>> {
    let mut ranges = Vec::new();
    let input_path = Path::new("ip-ranges.txt");
    
    // Read the entire file content
    let content = fs::read_to_string(input_path)
        .context("Failed to read IP ranges file")?;

    // Extract IP ranges from any format
    let extracted_ranges = extract_ip_ranges(&content);
    
    for (range_str, source) in extracted_ranges {
        match parse_ip_range(&range_str) {
            Ok(network) => ranges.push((network, source)),
            Err(e) => eprintln!("Warning: Failed to parse IP range '{}': {}", range_str, e),
        }
    }

    if ranges.is_empty() {
        anyhow::bail!("No valid IP ranges found in input file");
    }

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(Clear(ClearType::All));
    let _ = stdout.execute(cursor::MoveTo(0, 0));
    console_log(format!("Found {} valid IP ranges", ranges.len()));
    Ok(ranges)
}

async fn scan_range(
    network: Ipv4Net,
    location: String,
    client: Arc<reqwest::Client>,
    semaphore: Arc<Semaphore>,
    progress: Arc<ProgressBar>,
    model_writer: Arc<tokio::sync::Mutex<csv::Writer<std::fs::File>>>,
    endpoint_writer: Arc<tokio::sync::Mutex<csv::Writer<std::fs::File>>>,
) -> Vec<ScanResult> {
    let mut results = Vec::new();
    let mut futures = Vec::new();
    let mut last_scan = Instant::now();
    let mut scan_count = 0;
    
    for ip in network.hosts() {
        if STOP_SCAN.load(Ordering::Relaxed) {
            break;
        }

        while PAUSE_SCAN.load(Ordering::Relaxed) {
            progress.set_message("PAUSED");
            tokio::time::sleep(Duration::from_millis(100)).await;
            if STOP_SCAN.load(Ordering::Relaxed) {
                break;
            }
        }
        progress.set_message("");

        // Rate limiting
        scan_count += 1;
        if scan_count >= RATE_LIMIT_PER_SECOND {
            let elapsed = last_scan.elapsed();
            if elapsed < Duration::from_secs(1) {
                tokio::time::sleep(Duration::from_secs(1) - elapsed).await;
            }
            last_scan = Instant::now();
            scan_count = 0;
        }

        let ip = ip.to_string();
        let location = location.clone();
        let client = client.clone();
        let semaphore = semaphore.clone();
        let progress = progress.clone();
        let model_writer = model_writer.clone();
        let endpoint_writer = endpoint_writer.clone();

        futures.push(tokio::spawn(async move {
            let result = check_host(ip, location, &client, semaphore, model_writer, endpoint_writer).await;
            progress.inc(1);
            result
        }));

        // Process in smaller chunks to avoid memory buildup
        if futures.len() >= 500 {
            let chunk = futures.split_off(futures.len() - 500);
            let mut buffer = futures::stream::iter(chunk)
                .buffer_unordered(100)
                .collect::<Vec<_>>()
                .await;

            for result in buffer.drain(..) {
                if let Ok(Some(scan_result)) = result {
                    results.push(scan_result);
                }
            }
        }
    }

    // Process remaining futures
    let mut buffer = futures::stream::iter(futures)
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;

    for result in buffer.drain(..) {
        if let Ok(Some(scan_result)) = result {
            results.push(scan_result);
        }
    }

    results
}

fn setup_keyboard_handler() {
    std::thread::spawn(|| {
        while !STOP_SCAN.load(Ordering::Relaxed) {
            // Poll for keyboard events with a timeout
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
                    match code {
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            PAUSE_SCAN.store(true, Ordering::Relaxed);
                            console_log(style("Scan paused. Press 'r' to resume...").yellow().to_string());
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            PAUSE_SCAN.store(false, Ordering::Relaxed);
                            console_log(style("Scan resumed").green().to_string());
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            console_log(style("Exiting...").yellow().to_string());
                            STOP_SCAN.store(true, Ordering::Relaxed);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

mod disclaimer;
use disclaimer::display_disclaimer;

#[tokio::main]
async fn main() -> Result<()> {
    // Display disclaimer and check agreement
    if !display_disclaimer()? {
        return Ok(());
    }

    // Enable raw mode for keyboard input
    crossterm::terminal::enable_raw_mode()?;
    
    ctrlc::set_handler(|| {
        console_log(format!("{}",
            style("Stopping scan... Press Ctrl+C again to force quit").yellow()
        ));
        STOP_SCAN.store(true, Ordering::Relaxed);
    })?;

    let ranges = load_ranges()?;
    let total_ips: u64 = ranges.iter().map(|(net, _)| net.hosts().count() as u64).sum();
    
    // Print with proper alignment
    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::MoveTo(0, 1));
    
    console_log(format!("\n{}{}", 
        HEADER_STYLE,
        style("Public Ollama Finder").blue().bold()
    ));
    console_log(format!("{}Repository: {}", 
        LIST_ITEM_STYLE,
        style(REPO_URL).yellow()
    ));
    console_log(format!("{}Targets: {} IP ranges ({} total IPs)", 
        LIST_ITEM_STYLE,
        style(ranges.len()).cyan(),
        style(total_ips).cyan()
    ));
    console_log(format!("{}Port: {}", 
        LIST_ITEM_STYLE,
        style("11434 /api/tags").yellow()
    ));
    console_log(format!("{}Controls: {}", 
        LAST_ITEM_STYLE,
        style("[p]ause [r]esume [q]uit | Ctrl+C to stop").dim()
    ));
    console_log("".to_string()); // Empty line before progress bar

    setup_keyboard_handler();

    let progress = ProgressBar::new(total_ips);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {percent:>3}% • {pos:>9}/{len} IPs {msg}")?
            .progress_chars("█▓░"),
    );

    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .pool_max_idle_per_host(100)  // Reduced from 500
            .tcp_keepalive(Duration::from_secs(10))
            .build()?,
    );
    let semaphore = Arc::new(Semaphore::new(CONCURRENT_LIMIT));
    let progress = Arc::new(progress);
    
    let endpoint_file = OpenOptions::new().append(true).create(true).open("ollama_endpoints.csv")?;
    let mut endpoint_writer = csv::WriterBuilder::new().has_headers(false).from_writer(endpoint_file);
    if fs::metadata("ollama_endpoints.csv")?.len() == 0 {
        endpoint_writer.write_record(&["IP:Port", "Tags URL", "Status Code", "Location"])?;
    }
    let endpoint_writer = Arc::new(tokio::sync::Mutex::new(endpoint_writer));

    let model_file = OpenOptions::new().append(true).create(true).open("llm_models.csv")?;
    let mut model_writer = csv::WriterBuilder::new().has_headers(false).from_writer(model_file);
    if fs::metadata("llm_models.csv")?.len() == 0 {
        model_writer.write_record(&[
            "IP:Port", "Model Name", "Model", "Modified At", "Size", "Digest", 
            "Parent Model", "Format", "Family", "Parameter Size", "Quantization Level"
        ])?;
    }
    let model_writer = Arc::new(tokio::sync::Mutex::new(model_writer));

    let mut found_endpoints = Vec::new();

    for (network, location) in ranges {
        if STOP_SCAN.load(Ordering::Relaxed) {
            break;
        }

        let results = scan_range(
            network,
            location,
            client.clone(),
            semaphore.clone(),
            progress.clone(),
            model_writer.clone(),
            endpoint_writer.clone(),
        ).await;

        for result in results {
            found_endpoints.push(result.clone());
        }
    }

    progress.finish_and_clear();

    if !found_endpoints.is_empty() {
        console_log(style(format!("Found {} Ollama endpoints", found_endpoints.len())).green().to_string());
    }

    if STOP_SCAN.load(Ordering::Relaxed) {
        console_log(style("Scan stopped by user").yellow().to_string());
    } else {
        console_log(style("Scan completed!").green().bold().to_string());
    }

    // Cleanup raw mode at the end
    let result = async {
        // ...existing main function code...
        Ok(())
    }.await;
    
    crossterm::terminal::disable_raw_mode()?;
    result
}
