use serde::Serialize;
use std::process::Command as StdCommand;
use sysinfo::Disks;

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub status: String,
    pub detail: String,
    pub data: Option<serde_json::Value>,
}

// Run a command and capture its stdout, returning None on failure.
fn run_command_output(cmd: &str, args: &[&str]) -> Option<String> {
    StdCommand::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
}

#[tauri::command]
pub async fn check_environment(check_id: String) -> Result<CheckResult, String> {
    match check_id.as_str() {
        "os" => Ok(check_os()),
        "node" => Ok(check_node()),
        "npm" => Ok(check_npm()),
        "network" => check_network().await,
        "disk" => Ok(check_disk()),
        _ => Err(format!("Unknown check id: {}", check_id)),
    }
}

// Check OS type and version.
fn check_os() -> CheckResult {
    let os_type = std::env::consts::OS.to_string();
    let os_version = sysinfo::System::os_version().unwrap_or_else(|| "unknown".to_string());

    CheckResult {
        status: "passed".to_string(),
        detail: format!("{} {}", os_type, os_version),
        data: Some(serde_json::json!({
            "osType": os_type,
            "osVersion": os_version,
        })),
    }
}

// Check if Node.js is installed and return its version.
fn check_node() -> CheckResult {
    match run_command_output("node", &["--version"]) {
        Some(version) => CheckResult {
            status: "passed".to_string(),
            detail: format!("Node.js {}", version),
            data: Some(serde_json::json!({ "version": version })),
        },
        None => CheckResult {
            status: "failed".to_string(),
            detail: "Node.js is not installed".to_string(),
            data: None,
        },
    }
}

// Check if npm/pnpm is installed and return version info.
fn check_npm() -> CheckResult {
    let npm_version = run_command_output("npm", &["--version"]);
    let pnpm_version = run_command_output("pnpm", &["--version"]);

    match (&npm_version, &pnpm_version) {
        (Some(npm_v), Some(pnpm_v)) => CheckResult {
            status: "passed".to_string(),
            detail: format!("npm {} / pnpm {}", npm_v, pnpm_v),
            data: Some(serde_json::json!({
                "npmVersion": npm_v,
                "pnpmVersion": pnpm_v,
            })),
        },
        (Some(npm_v), None) => CheckResult {
            status: "passed".to_string(),
            detail: format!("npm {}", npm_v),
            data: Some(serde_json::json!({
                "npmVersion": npm_v,
            })),
        },
        (None, Some(pnpm_v)) => CheckResult {
            status: "warning".to_string(),
            detail: format!("npm not found, pnpm {}", pnpm_v),
            data: Some(serde_json::json!({
                "pnpmVersion": pnpm_v,
            })),
        },
        (None, None) => CheckResult {
            status: "failed".to_string(),
            detail: "Neither npm nor pnpm is installed".to_string(),
            data: None,
        },
    }
}

// Test network connectivity to npmmirror.com.
async fn check_network() -> Result<CheckResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    let result = client.get("https://npmmirror.com").send().await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => Ok(CheckResult {
            status: "passed".to_string(),
            detail: format!("Network OK ({}ms)", elapsed_ms),
            data: Some(serde_json::json!({ "latencyMs": elapsed_ms })),
        }),
        Err(e) => Ok(CheckResult {
            status: "failed".to_string(),
            detail: format!("Network check failed: {}", e),
            data: None,
        }),
    }
}

// Check disk space availability (require at least 1GB free).
fn check_disk() -> CheckResult {
    let sys_disks = Disks::new_with_refreshed_list();
    let min_required_gb: f64 = 1.0;

    // Find the root or primary disk, fallback to the largest disk
    let primary_disk = sys_disks
        .iter()
        .find(|d| {
            let mount = d.mount_point().to_string_lossy();
            mount == "/"
                || mount == "C:\\"
                || mount == "/System/Volumes/Data"
        })
        .or_else(|| {
            sys_disks.iter().max_by_key(|d| d.total_space())
        });

    match primary_disk {
        Some(disk) => {
            let available_gb = disk.available_space() as f64 / 1_073_741_824.0;
            let total_gb = disk.total_space() as f64 / 1_073_741_824.0;

            if available_gb >= min_required_gb {
                CheckResult {
                    status: "passed".to_string(),
                    detail: format!("{:.1}GB available of {:.1}GB", available_gb, total_gb),
                    data: Some(serde_json::json!({
                        "availableGb": (available_gb * 10.0).round() / 10.0,
                        "totalGb": (total_gb * 10.0).round() / 10.0,
                    })),
                }
            } else {
                CheckResult {
                    status: "failed".to_string(),
                    detail: format!(
                        "Only {:.1}GB available, at least {:.1}GB required",
                        available_gb, min_required_gb
                    ),
                    data: Some(serde_json::json!({
                        "availableGb": (available_gb * 10.0).round() / 10.0,
                        "totalGb": (total_gb * 10.0).round() / 10.0,
                    })),
                }
            }
        }
        None => CheckResult {
            status: "warning".to_string(),
            detail: "Could not determine primary disk".to_string(),
            data: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_os_returns_passed() {
        let result = check_os();
        assert_eq!(result.status, "passed");
        assert!(!result.detail.is_empty());
        let data = result.data.unwrap();
        assert!(data.get("osType").is_some());
        assert!(data.get("osVersion").is_some());
    }

    #[test]
    fn check_disk_returns_valid_status() {
        let result = check_disk();
        // Should be either "passed", "failed", or "warning"
        assert!(
            result.status == "passed"
                || result.status == "failed"
                || result.status == "warning"
        );
        assert!(!result.detail.is_empty());
    }

    #[test]
    fn check_disk_includes_gb_data_when_disk_found() {
        let result = check_disk();
        if result.status != "warning" {
            let data = result.data.unwrap();
            let available = data.get("availableGb").unwrap().as_f64().unwrap();
            let total = data.get("totalGb").unwrap().as_f64().unwrap();
            assert!(available >= 0.0);
            assert!(total > 0.0);
            assert!(available <= total);
        }
    }

    #[test]
    fn check_node_returns_passed_or_failed() {
        let result = check_node();
        assert!(result.status == "passed" || result.status == "failed");
    }

    #[test]
    fn check_npm_returns_valid_status() {
        let result = check_npm();
        assert!(
            result.status == "passed"
                || result.status == "warning"
                || result.status == "failed"
        );
    }

    #[test]
    fn run_command_output_returns_none_for_invalid_command() {
        let result = run_command_output("nonexistent_command_12345", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn run_command_output_returns_some_for_echo() {
        let result = run_command_output("echo", &["hello"]);
        assert_eq!(result, Some("hello".to_string()));
    }

    // run_command_output edge cases
    #[test]
    fn run_command_output_trims_whitespace() {
        // echo adds a newline, should be trimmed
        let result = run_command_output("echo", &["  test  "]);
        if let Some(val) = result {
            assert!(!val.ends_with('\n'), "Should not end with newline");
        }
    }

    #[test]
    fn run_command_output_returns_none_for_failing_command() {
        // `false` command always exits with non-zero
        let result = run_command_output("false", &[]);
        assert!(result.is_none(), "`false` command should return None");
    }

    // check_environment dispatch
    #[tokio::test]
    async fn check_environment_dispatches_os() {
        let result = check_environment("os".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "passed");
    }

    #[tokio::test]
    async fn check_environment_dispatches_node() {
        let result = check_environment("node".to_string()).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.status == "passed" || r.status == "failed");
    }

    #[tokio::test]
    async fn check_environment_dispatches_npm() {
        let result = check_environment("npm".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_environment_dispatches_disk() {
        let result = check_environment("disk".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_environment_rejects_unknown_id() {
        let result = check_environment("unknown_check".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown check id"));
    }

    #[tokio::test]
    async fn check_environment_rejects_empty_id() {
        let result = check_environment("".to_string()).await;
        assert!(result.is_err());
    }

    // check_os platform-specific validation
    #[test]
    fn check_os_detail_contains_platform_name() {
        let result = check_os();
        let os = std::env::consts::OS;
        assert!(
            result.detail.contains(os),
            "OS detail should contain '{}': {}", os, result.detail
        );
    }

    #[test]
    fn check_os_data_has_correct_os_type() {
        let result = check_os();
        let data = result.data.unwrap();
        let os_type = data.get("osType").unwrap().as_str().unwrap();
        assert_eq!(os_type, std::env::consts::OS);
    }

    // check_node version format
    #[test]
    fn check_node_version_format_if_installed() {
        let result = check_node();
        if result.status == "passed" {
            let data = result.data.unwrap();
            let version = data.get("version").unwrap().as_str().unwrap();
            assert!(version.starts_with('v'), "Node version should start with 'v': {}", version);
            assert!(result.detail.contains("Node.js"));
        }
    }

    #[test]
    fn check_node_failed_has_no_data() {
        let result = check_node();
        if result.status == "failed" {
            // On systems without Node, data should be None
            assert!(result.data.is_none());
            assert!(result.detail.contains("not installed"));
        }
    }

    // check_npm detail format
    #[test]
    fn check_npm_includes_version_numbers() {
        let result = check_npm();
        if result.status == "passed" {
            // Detail should contain at least "npm X.Y.Z"
            assert!(result.detail.contains("npm "), "Detail should contain npm version: {}", result.detail);
        }
    }

    // check_disk detail format
    #[test]
    fn check_disk_detail_contains_gb_units() {
        let result = check_disk();
        if result.status != "warning" {
            assert!(result.detail.contains("GB"), "Detail should contain GB units: {}", result.detail);
        }
    }

    // CheckResult struct validation
    #[test]
    fn check_result_serializes_correctly() {
        let result = CheckResult {
            status: "passed".to_string(),
            detail: "All good".to_string(),
            data: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"passed\""));
        assert!(json.contains("\"detail\":\"All good\""));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn check_result_serializes_with_none_data() {
        let result = CheckResult {
            status: "failed".to_string(),
            detail: "Error".to_string(),
            data: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"data\":null"));
    }
}
