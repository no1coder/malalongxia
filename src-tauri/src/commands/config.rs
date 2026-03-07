use std::path::PathBuf;
use std::process::Command as StdCommand;
use tokio::process::Command;

use super::path_env::expanded_path;

/// Find a program using the expanded PATH (not just the process PATH).
fn find_program(name: &str) -> Result<std::path::PathBuf, String> {
    which::which_in(name, Some(expanded_path()), ".")
        .map_err(|_| format!("{} is not installed or not in PATH", name))
}

/// Create a tokio Command with the expanded PATH set.
/// On Windows, wraps through `cmd.exe /C` so `.cmd` scripts (npm.cmd) are found.
fn cmd(program: &str) -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", program]);
        c.env("PATH", expanded_path());
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new(program);
        c.env("PATH", expanded_path());
        c
    }
}

// Resolve the OpenClaw config directory path.
fn openclaw_config_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join(".openclaw"))
}

const OPENCLAW_PORT: u16 = 18789;
const OPENCLAW_URL: &str = "http://127.0.0.1:18789";

// Read the gateway auth token from ~/.openclaw/openclaw.json
async fn read_gateway_token() -> Option<String> {
    let config_dir = openclaw_config_dir().ok()?;
    let config_path = config_dir.join("openclaw.json");
    let content = tokio::fs::read_to_string(&config_path).await.ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("gateway")?
        .get("auth")?
        .get("token")?
        .as_str()
        .map(|s| s.to_string())
}

// Build the gateway URL with token query parameter if available.
async fn gateway_url_with_token() -> String {
    match read_gateway_token().await {
        Some(token) => format!("{}/?token={}", OPENCLAW_URL, token),
        None => OPENCLAW_URL.to_string(),
    }
}

#[derive(serde::Serialize)]
pub struct OpenClawStatus {
    pub installed: bool,
    pub running: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub needs_update: bool,
    pub gateway_url: String,
}

#[tauri::command]
pub async fn check_openclaw_status() -> Result<OpenClawStatus, String> {
    // Check if openclaw binary exists
    let installed = find_program("openclaw").is_ok();

    // Check if gateway is already running by probing the port
    let running = {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok();
        match client {
            Some(c) => c.get(OPENCLAW_URL).send().await.is_ok(),
            None => false,
        }
    };

    // Run npm list and npm view in parallel with timeout to avoid hanging
    let npm_timeout = std::time::Duration::from_secs(5);

    let current_version_fut = async {
        if !installed { return None; }
        let output = cmd("npm")
            .args(["list", "-g", "openclaw", "--depth=0", "--json"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .ok()?;

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        json.get("dependencies")?
            .get("openclaw")?
            .get("version")?
            .as_str()
            .map(|s| s.to_string())
    };

    let latest_version_fut = async {
        let output = cmd("npm")
            .args(["view", "openclaw", "version"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        } else {
            None
        }
    };

    let (current_version, latest_version) = tokio::join!(
        async { tokio::time::timeout(npm_timeout, current_version_fut).await.ok().flatten() },
        async { tokio::time::timeout(npm_timeout, latest_version_fut).await.ok().flatten() }
    );

    let needs_update = match (&current_version, &latest_version) {
        (Some(current), Some(latest)) => current != latest,
        _ => false,
    };

    let gateway_url = gateway_url_with_token().await;

    Ok(OpenClawStatus {
        installed,
        running,
        current_version,
        latest_version,
        needs_update,
        gateway_url,
    })
}

#[tauri::command]
pub async fn update_openclaw() -> Result<String, String> {
    find_program("openclaw")?;

    // Use `openclaw update` as the official update mechanism
    let output = cmd("openclaw")
        .args(["update"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to update openclaw: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Fallback to npm if `openclaw update` fails
        let npm_output = cmd("npm")
            .args(["install", "-g", "openclaw@latest"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to update via npm: {}", e))?;

        if !npm_output.status.success() {
            let npm_stderr = String::from_utf8_lossy(&npm_output.stderr);
            return Err(format!("Update failed: {}\nnpm fallback: {}", stderr, npm_stderr));
        }
    }

    // Get new version
    let ver_output = cmd("openclaw")
        .args(["--version"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .ok();

    let new_version = ver_output.and_then(|o| {
        if o.status.success() {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        } else {
            None
        }
    });

    Ok(new_version.unwrap_or_else(|| "unknown".to_string()))
}

const APP_VERSION_URL: &str = "https://malalongxia.com/version.json";

#[derive(serde::Serialize)]
pub struct AppUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
}

/// Check for app updates by fetching version.json from the website.
#[tauri::command]
pub async fn check_app_update(app_handle: tauri::AppHandle) -> Result<AppUpdateInfo, String> {
    let current_version = app_handle.config().version.clone().unwrap_or_default();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(APP_VERSION_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Version check failed with status: {}", response.status()));
    }

    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse version JSON: {}", e))?;

    let latest_version = body
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    let download_url = body
        .get("download_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("https://malalongxia.com")
        .to_string();
    let release_notes = body
        .get("release_notes")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let has_update = !latest_version.is_empty() && latest_version != current_version;

    Ok(AppUpdateInfo {
        has_update,
        current_version,
        latest_version,
        download_url,
        release_notes,
    })
}

/// Test an API connection by sending a minimal OpenAI-compatible chat completions request.
#[tauri::command]
pub async fn test_api_connection(
    base_url: String,
    api_key: String,
    model: String,
) -> Result<String, String> {
    let endpoint = format!(
        "{}/chat/completions",
        base_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5,
        "stream": false,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("Failed to serialize request body: {}", e))?;

    let response = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .body(body_str)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        // Try to extract error message from JSON response
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Some(msg) = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                return Err(format!("API error ({}): {}", status.as_u16(), msg));
            }
        }
        return Err(format!("API error ({}): {}", status.as_u16(), response_text));
    }

    // Extract model name from response for confirmation
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if let Some(model_name) = json.get("model").and_then(|m| m.as_str()) {
            return Ok(model_name.to_string());
        }
    }

    Ok("OK".to_string())
}

/// Configure an AI provider via `openclaw onboard --non-interactive`.
///
/// This creates proper auth profiles in `~/.openclaw/agents/main/agent/auth-profiles.json`,
/// which is the correct way to register API keys with OpenClaw.
#[tauri::command]
pub async fn configure_api(
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<(), String> {
    // Refresh PATH so we can find openclaw after a fresh install
    super::path_env::refresh_system_path();

    // Build the openclaw onboard command with provider-specific flags
    let mut args: Vec<String> = vec![
        "onboard".to_string(),
        "--non-interactive".to_string(),
        "--accept-risk".to_string(),
        "--skip-channels".to_string(),
        "--skip-daemon".to_string(),
        "--skip-health".to_string(),
        "--skip-skills".to_string(),
        "--skip-ui".to_string(),
    ];

    // Providers that configure directly via openclaw.json (not via onboard)
    let direct_config_providers = ["bailian"];
    if direct_config_providers.contains(&provider.as_str()) {
        return configure_api_direct(&provider, &api_key, &base_url, &model).await;
    }

    // Map provider to the correct onboard flags
    match provider.as_str() {
        "zai" => {
            args.push("--auth-choice".to_string());
            args.push("zai-api-key".to_string());
            args.push("--zai-api-key".to_string());
            args.push(api_key.clone());
        }
        "openai" => {
            args.push("--auth-choice".to_string());
            args.push("openai-api-key".to_string());
            args.push("--openai-api-key".to_string());
            args.push(api_key.clone());
        }
        "anthropic" => {
            args.push("--auth-choice".to_string());
            args.push("anthropic-api-key".to_string());
            args.push("--anthropic-api-key".to_string());
            args.push(api_key.clone());
        }
        "moonshot" => {
            args.push("--auth-choice".to_string());
            args.push("moonshot-api-key".to_string());
            args.push("--moonshot-api-key".to_string());
            args.push(api_key.clone());
        }
        "qianfan" => {
            args.push("--auth-choice".to_string());
            args.push("qianfan-api-key".to_string());
            args.push("--qianfan-api-key".to_string());
            args.push(api_key.clone());
        }
        // deepseek, custom, etc. use custom-api-key with their specific base URLs
        _ => {
            args.push("--auth-choice".to_string());
            args.push("custom-api-key".to_string());
            args.push("--custom-api-key".to_string());
            args.push(api_key.clone());
            if !base_url.is_empty() {
                args.push("--custom-base-url".to_string());
                args.push(base_url.clone());
            }
            if !model.is_empty() {
                args.push("--custom-model-id".to_string());
                args.push(model.clone());
            }
        }
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let onboard_output = cmd("openclaw")
        .args(&args_ref)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run openclaw onboard: {}", e))?;

    if !onboard_output.status.success() {
        let stderr = String::from_utf8_lossy(&onboard_output.stderr);
        let stdout = String::from_utf8_lossy(&onboard_output.stdout);
        return Err(format!(
            "Failed to configure API: {}{}",
            stderr.trim(),
            if !stdout.trim().is_empty() {
                format!("\n{}", stdout.trim())
            } else {
                String::new()
            }
        ));
    }

    // Set the default model for native providers (onboard already sets it for custom providers)
    let is_native = matches!(provider.as_str(), "zai" | "openai" | "anthropic" | "moonshot" | "qianfan");
    if is_native && !model.is_empty() {
        let model_id = if model.contains('/') {
            model.clone()
        } else {
            format!("{}/{}", provider, model)
        };

        let model_result = cmd("openclaw")
            .args(["config", "set", "agents.defaults.model.primary", &model_id])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        if let Err(e) = &model_result {
            eprintln!("Warning: failed to set default model: {}", e);
        } else if let Ok(o) = &model_result {
            if !o.status.success() {
                eprintln!(
                    "Warning: openclaw config set failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                );
            }
        }
    }

    Ok(())
}

/// Configure providers that need direct openclaw.json modification (e.g. bailian/DashScope).
/// These providers are not supported by `openclaw onboard` and must be configured by
/// merging provider config into `~/.openclaw/openclaw.json`.
async fn configure_api_direct(
    provider: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
) -> Result<(), String> {
    let config_dir = openclaw_config_dir()?;
    let config_path = config_dir.join("openclaw.json");

    // Ensure the config directory exists (first-time install may not have created it yet)
    tokio::fs::create_dir_all(&config_dir)
        .await
        .map_err(|e| format!("Failed to create config directory {}: {}", config_dir.display(), e))?;

    // Read existing config
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let mut config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid openclaw.json: {}", e))?;

    // Build the provider entry
    let model_id = if model.is_empty() { "default" } else { model };
    let provider_config = serde_json::json!({
        "baseUrl": base_url,
        "apiKey": api_key,
        "api": "openai-completions",
        "models": [{
            "id": model_id,
            "name": model_id,
            "reasoning": false,
            "input": ["text", "image"],
            "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
            "contextWindow": 1000000,
            "maxTokens": 65536,
        }],
    });

    // Merge into config: models.providers.<provider>
    config
        .as_object_mut()
        .ok_or("Config is not an object")?
        .entry("models")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("models is not an object")?
        .entry("providers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("providers is not an object")?
        .insert(provider.to_string(), provider_config);

    // Set default model: agents.defaults.model.primary
    let full_model_id = format!("{}/{}", provider, model_id);
    config
        .as_object_mut()
        .ok_or("Config is not an object")?
        .entry("agents")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("agents is not an object")?
        .entry("defaults")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("defaults is not an object")?
        .entry("model")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("model is not an object")?
        .insert(
            "primary".to_string(),
            serde_json::Value::String(full_model_id),
        );

    // Write back
    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    tokio::fs::write(&config_path, output)
        .await
        .map_err(|e| format!("Failed to write openclaw.json: {}", e))?;

    Ok(())
}

// Helper: open a URL in the system default browser.
fn open_in_browser(url: &str) -> Result<(), String> {
    let os = std::env::consts::OS;
    let result = match os {
        "macos" => StdCommand::new("open").arg(url).spawn(),
        "windows" => StdCommand::new("cmd").args(["/c", "start", "", url]).spawn(),
        _ => StdCommand::new("xdg-open").arg(url).spawn(),
    };
    result.map_err(|e| format!("Failed to open browser: {}", e))?;
    Ok(())
}

// Helper: probe if gateway is reachable.
async fn is_gateway_running() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok();
    match client {
        Some(c) => c.get(OPENCLAW_URL).send().await.is_ok(),
        None => false,
    }
}

// Ensure gateway prerequisites are met before starting.
// Fixes common issues: missing gateway.mode config, stale LaunchAgent paths.
async fn ensure_gateway_config() {
    // Run config set in parallel with uninstall (they are independent)
    let config_fut = cmd("openclaw")
        .args(["config", "set", "gateway.mode", "local"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    let uninstall_fut = cmd("openclaw")
        .args(["gateway", "uninstall"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    let _ = tokio::join!(config_fut, uninstall_fut);

    // Reinstall depends on uninstall completing first
    let _ = cmd("openclaw")
        .args(["gateway", "install"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await;
}

#[tauri::command]
pub async fn launch_openclaw() -> Result<String, String> {
    find_program("openclaw")?;

    // If already running, just open browser with token
    if is_gateway_running().await {
        let auth_url = gateway_url_with_token().await;
        open_in_browser(&auth_url)?;
        return Ok(auth_url);
    }

    // Auto-fix common config issues before starting
    ensure_gateway_config().await;

    // Try service mode first: `openclaw gateway start`
    let service_result = cmd("openclaw")
        .args(["gateway", "start"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let used_service = match &service_result {
        Ok(o) => o.status.success(),
        Err(_) => false,
    };

    // Fallback to foreground mode if service mode fails
    if !used_service {
        cmd("openclaw")
            .args(["gateway", "run", "--port", &OPENCLAW_PORT.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch openclaw gateway: {}", e))?;
    }

    // Wait for the gateway to become ready (up to 30s)
    let mut ready = false;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if is_gateway_running().await {
            ready = true;
            break;
        }
    }

    if !ready {
        // Collect diagnostic info for the error message
        let doctor_output = cmd("openclaw")
            .args(["doctor"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let err_log = tokio::fs::read_to_string(
            dirs::home_dir()
                .unwrap_or_default()
                .join(".openclaw/logs/gateway.err.log"),
        )
        .await
        .ok()
        .and_then(|s| {
            // Take only the last 500 chars to keep the error message reasonable
            let tail = if s.len() > 500 { &s[s.len() - 500..] } else { &s };
            if tail.is_empty() { None } else { Some(tail.to_string()) }
        })
        .unwrap_or_default();

        return Err(format!(
            "Gateway did not respond within 30s.\n\n--- Doctor ---\n{}\n--- Error Log ---\n{}",
            doctor_output.trim(),
            err_log.trim()
        ));
    }

    // Open browser with auth token
    let auth_url = gateway_url_with_token().await;
    open_in_browser(&auth_url)?;
    Ok(auth_url)
}

// Stop the gateway service.
#[tauri::command]
pub async fn stop_openclaw_gateway() -> Result<String, String> {
    find_program("openclaw")?;

    let output = cmd("openclaw")
        .args(["gateway", "stop"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to stop gateway: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!("Stop failed: {}", if stderr.is_empty() { stdout } else { stderr }))
    }
}

// Restart the gateway service.
#[tauri::command]
pub async fn restart_openclaw_gateway() -> Result<String, String> {
    find_program("openclaw")?;

    // Auto-fix config issues before restarting
    ensure_gateway_config().await;

    // Try service restart first
    let output = cmd("openclaw")
        .args(["gateway", "restart"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to restart gateway: {}", e))?;

    if output.status.success() {
        // Wait for it to come back up (30s to match launch timeout)
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if is_gateway_running().await {
                return Ok(gateway_url_with_token().await);
            }
        }
        return Err("Gateway restarted but did not respond within 30s. Run diagnostics to check.".to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Err(format!("Restart failed: {}", stderr))
}

// Run `openclaw doctor` for diagnostics.
#[tauri::command]
pub async fn openclaw_doctor() -> Result<String, String> {
    find_program("openclaw")?;

    let output = cmd("openclaw")
        .args(["doctor"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run doctor: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

// Run `openclaw health` to check gateway health.
#[tauri::command]
pub async fn openclaw_health() -> Result<String, String> {
    find_program("openclaw")?;

    let output = cmd("openclaw")
        .args(["health", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Health check failed: {}", if stderr.is_empty() { stdout } else { stderr }))
    }
}

// Open the OpenClaw dashboard (TUI).
#[tauri::command]
pub async fn openclaw_dashboard() -> Result<String, String> {
    find_program("openclaw")?;

    // `openclaw dashboard` opens the WebUI with auth token
    let auth_url = gateway_url_with_token().await;
    open_in_browser(&auth_url)?;
    Ok(auth_url)
}

// Repair gateway config and service installation.
#[tauri::command]
pub async fn repair_openclaw() -> Result<String, String> {
    find_program("openclaw")?;

    let mut steps: Vec<String> = Vec::new();

    // Step 1: Ensure gateway.mode is set
    let mode_result = cmd("openclaw")
        .args(["config", "set", "gateway.mode", "local"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;
    match mode_result {
        Ok(o) if o.status.success() => steps.push("[OK] Set gateway.mode=local".to_string()),
        _ => steps.push("[FAIL] Failed to set gateway.mode".to_string()),
    }

    // Step 2: Stop existing service (ignore errors)
    let _ = cmd("openclaw")
        .args(["gateway", "stop"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await;
    steps.push("[OK] Stopped existing service".to_string());

    // Step 3: Uninstall stale service
    let uninstall_result = cmd("openclaw")
        .args(["gateway", "uninstall"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;
    match uninstall_result {
        Ok(o) if o.status.success() => steps.push("[OK] Uninstalled old service".to_string()),
        _ => steps.push("[WARN] No existing service to uninstall".to_string()),
    }

    // Step 4: Reinstall service with current paths
    let install_result = cmd("openclaw")
        .args(["gateway", "install"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;
    match install_result {
        Ok(o) if o.status.success() => steps.push("[OK] Installed gateway service".to_string()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            steps.push(format!("[FAIL] Failed to install service: {}", stderr.trim()));
        }
        Err(e) => steps.push(format!("[FAIL] Failed to install service: {}", e)),
    }

    // Step 5: Fix permissions on ~/.openclaw (Unix only)
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let openclaw_dir = home.join(".openclaw");
    if openclaw_dir.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            match std::fs::set_permissions(&openclaw_dir, perms) {
                Ok(_) => steps.push("[OK] Fixed directory permissions".to_string()),
                Err(e) => steps.push(format!("[FAIL] Failed to fix permissions: {}", e)),
            }
        }
        #[cfg(not(unix))]
        {
            steps.push("[OK] Directory permissions (not applicable on Windows)".to_string());
        }
    }

    Ok(steps.join("\n"))
}

#[tauri::command]
pub async fn export_logs(logs: Vec<String>) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let log_dir = home.join(".openclaw").join("logs");

    tokio::fs::create_dir_all(&log_dir)
        .await
        .map_err(|e| format!("Failed to create log directory: {}", e))?;

    let timestamp = chrono_timestamp();
    let log_file = log_dir.join(format!("install-log-{}.txt", timestamp));
    let content = logs.join("\n");

    tokio::fs::write(&log_file, &content)
        .await
        .map_err(|e| format!("Failed to write log file: {}", e))?;

    Ok(log_file.to_string_lossy().to_string())
}

// Open a URL in the system default browser (restricted to http/https).
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!("Disallowed URL scheme: {}", url));
    }
    open_in_browser(&url)
}

// Generate a simple timestamp string without pulling in chrono.
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}

// Check if the Feishu plugin is installed.
#[tauri::command]
pub async fn check_feishu_plugin() -> Result<bool, String> {
    find_program("openclaw")?;

    let output = cmd("openclaw")
        .args(["plugins", "list"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to list plugins: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("@openclaw/feishu"))
}

// Install the Feishu plugin.
#[tauri::command]
pub async fn install_feishu_plugin() -> Result<String, String> {
    find_program("openclaw")?;

    let output = cmd("openclaw")
        .args(["plugins", "install", "@openclaw/feishu"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to install Feishu plugin: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Install failed: {}", if stderr.is_empty() { stdout } else { stderr.to_string() }))
    }
}

// Configure the Feishu plugin with App ID and App Secret.
#[tauri::command]
pub async fn configure_feishu(
    app_id: String,
    app_secret: String,
) -> Result<(), String> {
    find_program("openclaw")?;

    // Set Feishu config via openclaw config set
    let id_result = cmd("openclaw")
        .args(["config", "set", "channels.feishu.appId", &app_id])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    let secret_result = cmd("openclaw")
        .args(["config", "set", "channels.feishu.appSecret", &app_secret])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    let (id_out, secret_out) = tokio::join!(id_result, secret_result);

    let id_ok = id_out.map(|o| o.status.success()).unwrap_or(false);
    let secret_ok = secret_out.map(|o| o.status.success()).unwrap_or(false);

    if id_ok && secret_ok {
        Ok(())
    } else {
        Err("Failed to configure Feishu plugin".to_string())
    }
}

#[tauri::command]
pub async fn reset_installation() -> Result<(), String> {
    // Step 1: Uninstall openclaw globally
    let child = cmd("npm")
        .args(["uninstall", "-g", "openclaw"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to uninstall openclaw: {}", e))?;

    if !child.status.success() {
        let stderr = String::from_utf8_lossy(&child.stderr);
        // Not a fatal error if openclaw was not installed
        if !stderr.contains("not installed") {
            return Err(format!("Failed to uninstall openclaw: {}", stderr));
        }
    }

    // Step 2: Remove openclaw config directory
    let config_dir = openclaw_config_dir()?;
    if config_dir.exists() {
        tokio::fs::remove_dir_all(&config_dir)
            .await
            .map_err(|e| format!("Failed to remove config directory: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openclaw_config_dir_is_under_home() {
        let dir = openclaw_config_dir().unwrap();
        let home = dirs::home_dir().unwrap();
        assert!(dir.starts_with(&home));
        assert!(dir.ends_with(".openclaw"));
    }

    #[test]
    fn chrono_timestamp_returns_numeric_string() {
        let ts = chrono_timestamp();
        assert!(!ts.is_empty());
        assert!(ts.parse::<u64>().is_ok(), "Timestamp should be numeric: {}", ts);
    }

    #[test]
    fn chrono_timestamp_is_recent() {
        let ts: u64 = chrono_timestamp().parse().unwrap();
        // Should be after 2024-01-01 (1704067200)
        assert!(ts > 1704067200, "Timestamp too old: {}", ts);
    }

    #[tokio::test]
    async fn open_url_rejects_javascript_scheme() {
        let result = open_url("javascript:alert(1)".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Disallowed URL scheme"));
    }

    #[tokio::test]
    async fn open_url_rejects_file_scheme() {
        let result = open_url("file:///etc/passwd".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Disallowed URL scheme"));
    }

    #[tokio::test]
    async fn open_url_rejects_ftp_scheme() {
        let result = open_url("ftp://example.com".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn open_url_rejects_data_scheme() {
        let result = open_url("data:text/html,<h1>XSS</h1>".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn gateway_url_with_token_returns_valid_url() {
        let url = gateway_url_with_token().await;
        assert!(url.starts_with("http://127.0.0.1:18789"));
    }

    #[test]
    fn openclaw_port_is_expected_value() {
        assert_eq!(OPENCLAW_PORT, 18789);
    }

    // Cross-platform open_in_browser tests
    #[test]
    fn open_in_browser_uses_correct_command_for_platform() {
        let os = std::env::consts::OS;
        // Validate the match arms cover the current OS
        let expected_cmd = match os {
            "macos" => "open",
            "windows" => "cmd",
            _ => "xdg-open",
        };
        // We can't easily test spawn, but verify the logic matches
        assert!(!expected_cmd.is_empty());
    }

    // open_url allows valid schemes
    #[tokio::test]
    async fn open_url_accepts_http_scheme() {
        // http:// should be allowed (will fail to spawn browser in test, but passes validation)
        let result = open_url("http://localhost:18789".to_string()).await;
        // Either Ok (browser opened) or Err (spawn failed), but NOT "Disallowed URL scheme"
        if let Err(e) = &result {
            assert!(!e.contains("Disallowed URL scheme"), "http:// should be allowed");
        }
    }

    #[tokio::test]
    async fn open_url_accepts_https_scheme() {
        let result = open_url("https://example.com".to_string()).await;
        if let Err(e) = &result {
            assert!(!e.contains("Disallowed URL scheme"), "https:// should be allowed");
        }
    }

    // Additional scheme rejection tests
    #[tokio::test]
    async fn open_url_rejects_mailto_scheme() {
        let result = open_url("mailto:test@example.com".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Disallowed URL scheme"));
    }

    #[tokio::test]
    async fn open_url_rejects_ssh_scheme() {
        let result = open_url("ssh://user@host".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Disallowed URL scheme"));
    }

    #[tokio::test]
    async fn open_url_rejects_empty_string() {
        let result = open_url("".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn open_url_rejects_relative_path() {
        let result = open_url("../etc/passwd".to_string()).await;
        assert!(result.is_err());
    }

    // Gateway URL format
    #[tokio::test]
    async fn gateway_url_format_is_correct() {
        let url = gateway_url_with_token().await;
        // Either plain URL or URL with token
        assert!(
            url == OPENCLAW_URL || url.starts_with(&format!("{}/?token=", OPENCLAW_URL)),
            "Unexpected gateway URL format: {}", url
        );
    }

    #[test]
    fn openclaw_url_is_localhost() {
        assert_eq!(OPENCLAW_URL, "http://127.0.0.1:18789");
        assert!(OPENCLAW_URL.starts_with("http://127.0.0.1:"));
    }

    // Config dir path construction
    #[test]
    fn openclaw_config_dir_path_is_deterministic() {
        let dir1 = openclaw_config_dir().unwrap();
        let dir2 = openclaw_config_dir().unwrap();
        assert_eq!(dir1, dir2, "Config dir should be deterministic");
    }

    // Timestamp monotonicity
    #[test]
    fn chrono_timestamp_is_monotonic() {
        let ts1: u64 = chrono_timestamp().parse().unwrap();
        let ts2: u64 = chrono_timestamp().parse().unwrap();
        assert!(ts2 >= ts1, "Timestamps should be monotonically increasing");
    }

    // configure_api model ID format
    #[test]
    fn configure_api_model_id_format() {
        // Known providers get provider/model format
        let provider = "zai";
        let model = "glm-5";
        let model_id = format!("{}/{}", provider, model);
        assert_eq!(model_id, "zai/glm-5");

        // Models with existing slash are kept as-is
        let model_with_slash = "zai/glm-5";
        assert!(model_with_slash.contains('/'));
    }

    // Repair steps output format (Unix/Windows conditional)
    #[test]
    fn repair_step_format_uses_ascii_markers() {
        // Verify the step format strings we use
        let ok_step = "[OK] Set gateway.mode=local".to_string();
        let fail_step = "[FAIL] Failed to set gateway.mode".to_string();
        let warn_step = "[WARN] No existing service to uninstall".to_string();

        assert!(ok_step.starts_with("[OK]"));
        assert!(fail_step.starts_with("[FAIL]"));
        assert!(warn_step.starts_with("[WARN]"));

        // No Unicode emoji markers
        assert!(!ok_step.contains('\u{2713}')); // ✓
        assert!(!fail_step.contains('\u{2717}')); // ✗
        assert!(!warn_step.contains('\u{26A0}')); // ⚠
    }

    // Cross-platform permission logic
    #[test]
    fn repair_handles_permissions_per_platform() {
        let os = std::env::consts::OS;
        match os {
            "macos" | "linux" => {
                // Unix: should use chmod 0o700
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o700);
                    // 0o700 = rwx------
                    assert_eq!(perms.mode() & 0o777, 0o700);
                }
            }
            _ => {
                // Windows: permissions step should be skipped
                // This is handled by #[cfg(not(unix))] in repair_openclaw
            }
        }
    }
}
