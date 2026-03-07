use serde::Serialize;
use tauri::{Emitter, Manager, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::path_env::{expanded_path, refresh_system_path};

const NODE_VERSION: &str = "v22.14.0";

/// Check if a bundled resource file exists in the app's resource directory.
/// Full edition bundles Node.js and Git archives; Lite edition has none.
/// Returns the path if the file exists, None otherwise.
fn bundled_resource(app: &tauri::AppHandle, subdir: &str, filename: &str) -> Option<std::path::PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    let path = resource_dir.join("resources").join(subdir).join(filename);
    if path.exists() { Some(path) } else { None }
}

/// Get the bundled Node.js archive path for the current platform.
fn bundled_node_archive(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let filename = match os {
        "macos" => {
            let arch_str = if arch == "aarch64" { "arm64" } else { "x64" };
            format!("node-{}-darwin-{}.tar.gz", NODE_VERSION, arch_str)
        }
        "windows" => {
            let arch_str = if arch == "aarch64" { "arm64" } else { "x64" };
            format!("node-{}-win-{}.zip", NODE_VERSION, arch_str)
        }
        _ => return None,
    };
    bundled_resource(app, "node", &filename)
}

/// Get the bundled Git archive path for the current platform.
fn bundled_git_archive(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match os {
        "windows" => {
            let filename = if arch == "aarch64" {
                "MinGit-2.53.0-arm64.zip"
            } else {
                "MinGit-2.53.0-64-bit.zip"
            };
            bundled_resource(app, "git", filename)
        }
        "macos" => {
            let codename = detect_macos_codename();
            let filename = if arch == "aarch64" {
                format!("git-2.53.0-arm64_{}.bottle.tar.gz", codename)
            } else {
                format!("git-2.53.0-{}.bottle.tar.gz", codename)
            };
            bundled_resource(app, "git", &filename)
        }
        _ => None,
    }
}

// Create a tokio Command with expanded PATH for finding node/npm in packaged apps.
// On Windows, wraps the call through `cmd.exe /C` so that `.cmd` scripts (like npm.cmd)
// are resolved automatically—Rust's Command::new won't find .cmd files on its own.
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

#[derive(Debug, Serialize)]
pub struct InstallResult {
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct NodeVerifyResult {
    pub node_available: bool,
    pub npm_available: bool,
    pub node_version: Option<String>,
    pub npm_version: Option<String>,
}

// Verify that node and npm are actually callable after installation.
// Retries a few times with short delays to allow PATH propagation (especially on Windows).
async fn post_install_verify(window: &Window, channel: &str) -> Result<(), String> {
    emit_log(window, channel, "Verifying Node.js installation...");

    for attempt in 1..=5 {
        // Refresh PATH from system registry (Windows) before each attempt
        refresh_system_path();

        let node_ok = cmd("node")
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        let npm_ok = cmd("npm")
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        if node_ok && npm_ok {
            emit_log(window, channel, "Node.js and npm verified successfully.");
            return Ok(());
        }

        if attempt < 5 {
            emit_log(
                window,
                channel,
                &format!(
                    "Verification attempt {}/5: node={}, npm={}. Retrying...",
                    attempt,
                    if node_ok { "ok" } else { "not found" },
                    if npm_ok { "ok" } else { "not found" },
                ),
            );
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    Err("Node.js or npm is not available after installation. Please restart the app and try again.".to_string())
}

/// Frontend-callable command to verify node/npm availability.
/// Used as a gate before proceeding to OpenClaw install step.
#[tauri::command]
pub async fn verify_node_npm() -> Result<NodeVerifyResult, String> {
    // Refresh PATH so we pick up any recent installations
    refresh_system_path();

    let node_output = cmd("node")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let npm_output = cmd("npm")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let (node_available, node_version) = match node_output {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (true, if v.is_empty() { None } else { Some(v) })
        }
        _ => (false, None),
    };

    let (npm_available, npm_version) = match npm_output {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (true, if v.is_empty() { None } else { Some(v) })
        }
        _ => (false, None),
    };

    Ok(NodeVerifyResult {
        node_available,
        npm_available,
        node_version,
        npm_version,
    })
}

// Emit a progress event to the frontend with a channel prefix.
fn emit_progress(window: &Window, channel: &str, percent: u32, message: &str) {
    let payload = serde_json::json!({
        "percent": percent,
        "message": message,
    });
    let _ = window.emit(&format!("{}-progress", channel), payload);
}

// Emit a log line to the frontend with a channel prefix.
fn emit_log(window: &Window, channel: &str, line: &str) {
    // Strip ANSI escape codes (colors, cursor movement) from log lines
    let clean = strip_ansi_codes(line);
    let _ = window.emit(&format!("{}-log", channel), clean);
}

// Remove ANSI escape sequences from a string (e.g. \x1b[31m, \x1b[0m).
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC + '[' + params + final byte
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

// Download a file with progress reporting via Tauri events.
// Automatically retries up to 3 times on transient failures.
async fn download_with_progress(
    window: &Window,
    channel: &str,
    url: &str,
    dest: &str,
    progress_start: u32,
    progress_end: u32,
) -> Result<(), String> {
    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        if attempt > 1 {
            emit_log(window, channel, &format!("Download retry {}/{}...", attempt, max_retries));
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        match download_once(window, channel, url, dest, progress_start, progress_end).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                emit_log(window, channel, &format!("Download attempt {} failed: {}", attempt, e));
                last_error = e;
                let _ = tokio::fs::remove_file(dest).await;
            }
        }
    }

    Err(format!("Download failed after {} attempts: {}", max_retries, last_error))
}

// Single download attempt with progress reporting.
async fn download_once(
    window: &Window,
    channel: &str,
    url: &str,
    dest: &str,
    progress_start: u32,
    progress_end: u32,
) -> Result<(), String> {
    emit_log(window, channel, &format!("Downloading from {}", url));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("Failed to create file {}: {}", dest, e))?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    let mut last_emitted_percent: u32 = progress_start;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let fraction = downloaded as f64 / total_size as f64;
            let percent =
                progress_start + ((progress_end - progress_start) as f64 * fraction) as u32;
            if percent > last_emitted_percent {
                let downloaded_mb = downloaded as f64 / 1_048_576.0;
                let total_mb = total_size as f64 / 1_048_576.0;
                emit_progress(
                    window,
                    channel,
                    percent,
                    &format!("Downloading... {:.1}MB / {:.1}MB", downloaded_mb, total_mb),
                );
                last_emitted_percent = percent;
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Flush error: {}", e))?;

    let downloaded_mb = downloaded as f64 / 1_048_576.0;
    emit_log(window, channel, &format!("Download complete: {:.1}MB", downloaded_mb));

    Ok(())
}

// Stream stdout and stderr of a child process to the frontend via events.
async fn stream_child_output(
    window: &Window,
    channel: &str,
    mut child: tokio::process::Child,
) -> Result<(), String> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let w1 = window.clone();
    let ch1 = channel.to_string();
    let stdout_handle = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                emit_log(&w1, &ch1, &line);
            }
        }
    });

    let w2 = window.clone();
    let ch2 = channel.to_string();
    let stderr_handle = tokio::spawn(async move {
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                emit_log(&w2, &ch2, &line);
            }
        }
    });

    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    // 10-minute timeout to prevent processes from hanging forever
    let status = tokio::time::timeout(
        std::time::Duration::from_secs(600),
        child.wait(),
    )
    .await
    .map_err(|_| "Process timed out after 10 minutes".to_string())?
    .map_err(|e| format!("Failed to wait for process: {}", e))?;

    if !status.success() {
        return Err(format!("Process exited with status: {}", status));
    }
    Ok(())
}

// Install portable Git (MinGit) on Windows, or git via package manager on other platforms.
// MinGit is the official lightweight Git for Windows distribution (~30MB).
// Detect macOS codename from version number for Homebrew bottle matching.
#[cfg(target_os = "macos")]
fn detect_macos_codename() -> &'static str {
    let version = sysinfo::System::os_version().unwrap_or_default();
    let major: u32 = version.split('.').next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    match major {
        15 => "sequoia",
        14 => "sonoma",
        13 => "ventura",
        12 => "monterey",
        _ => "sonoma", // fallback to sonoma
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_macos_codename() -> &'static str {
    "sonoma"
}

async fn install_portable_git(window: &Window, channel: &str, app: &tauri::AppHandle) -> Result<(), String> {
    let os = std::env::consts::OS;

    match os {
        "windows" => {
            let arch = std::env::consts::ARCH;

            let tmp_zip = std::env::temp_dir().join("mingit.zip");
            let tmp_str = tmp_zip.to_string_lossy().to_string();

            // Check for bundled Git archive first (Full edition)
            if let Some(bundled_path) = bundled_git_archive(app) {
                emit_log(window, channel, "Using bundled portable Git (offline)...");
                emit_progress(window, channel, 2, "Extracting bundled Git...");
                // Copy bundled archive to temp for consistent extraction flow
                tokio::fs::copy(&bundled_path, &tmp_zip)
                    .await
                    .map_err(|e| format!("Failed to copy bundled Git: {}", e))?;
            } else {
                // Download MinGit from our own site (primary) with npmmirror fallback
                let git_version = "2.53.0";
                let filename = if arch == "aarch64" {
                    format!("MinGit-{}-arm64.zip", git_version)
                } else {
                    format!("MinGit-{}-64-bit.zip", git_version)
                };
                let primary_url = format!("https://malalongxia.com/downloads/{}", filename);
                let fallback_url = format!(
                    "https://registry.npmmirror.com/-/binary/git-for-windows/v{}.windows.1/{}",
                    git_version, filename
                );

                emit_progress(window, channel, 2, "Downloading portable Git...");
                let download_result = download_with_progress(window, channel, &primary_url, &tmp_str, 2, 8).await;
                if download_result.is_err() {
                    emit_log(window, channel, "Primary download failed, trying npmmirror fallback...");
                    download_with_progress(window, channel, &fallback_url, &tmp_str, 2, 8).await?;
                }
            }

            // Extract to %LOCALAPPDATA%\Programs\MinGit
            let local_app_data = std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| {
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join("AppData")
                        .join("Local")
                        .to_string_lossy()
                        .to_string()
                });
            let git_dir = std::path::PathBuf::from(&local_app_data)
                .join("Programs")
                .join("MinGit");

            emit_log(window, channel, &format!("Extracting MinGit to {} ...", git_dir.display()));

            // Create destination dir before extraction
            let _ = tokio::fs::create_dir_all(&git_dir).await;

            // Try tar.exe first (built-in on Windows 10 1803+), then PowerShell fallback
            let tar_result = Command::new("tar")
                .args(["-xf", &tmp_str, "-C", &git_dir.to_string_lossy()])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match tar_result {
                Ok(child) => {
                    stream_child_output(window, channel, child).await?;
                }
                Err(_) => {
                    // Fallback: PowerShell Expand-Archive
                    emit_log(window, channel, "tar not available, trying PowerShell...");
                    let extract_cmd = format!(
                        "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
                        tmp_str,
                        git_dir.display()
                    );
                    let ps_paths = [
                        "powershell.exe".to_string(),
                        format!("{}\\WindowsPowerShell\\v1.0\\powershell.exe",
                            std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())),
                    ];
                    let mut ps_child = None;
                    for ps in &ps_paths {
                        if let Ok(c) = Command::new(ps)
                            .args(["-NoProfile", "-Command", &extract_cmd])
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .spawn()
                        {
                            ps_child = Some(c);
                            break;
                        }
                    }
                    let child = ps_child.ok_or_else(|| {
                        "Failed to extract MinGit: neither tar nor PowerShell found".to_string()
                    })?;
                    stream_child_output(window, channel, child).await?;
                }
            }

            // Add MinGit to current process PATH so subsequent commands find git
            let git_cmd_dir = git_dir.join("cmd");
            let current_path = std::env::var("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{};{}", git_cmd_dir.display(), current_path));

            // Verify git works
            let git_check = Command::new("cmd")
                .args(["/C", "git", "--version"])
                .env("PATH", expanded_path())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !git_check {
                return Err("Failed to install portable Git. Please install Git manually.".to_string());
            }

            // Cleanup
            let _ = tokio::fs::remove_file(&tmp_zip).await;
            emit_log(window, channel, "Portable Git installed successfully.");
            Ok(())
        }
        "macos" => {
            emit_log(window, channel, "Git not found. Installing Git...");

            let arch = std::env::consts::ARCH;
            let git_version = "2.53.0";
            let macos_codename = detect_macos_codename();
            let filename = if arch == "aarch64" {
                format!("git-{}-arm64_{}.bottle.tar.gz", git_version, macos_codename)
            } else {
                format!("git-{}-{}.bottle.tar.gz", git_version, macos_codename)
            };
            emit_log(window, channel, &format!("Detected macOS variant: {}", macos_codename));

            let tmp_tarball = std::env::temp_dir().join(&filename);
            let tmp_str = tmp_tarball.to_string_lossy().to_string();

            // Check for bundled Git archive first (Full edition)
            if let Some(bundled_path) = bundled_git_archive(app) {
                emit_log(window, channel, "Using bundled Git (offline)...");
                emit_progress(window, channel, 2, "Extracting bundled Git...");
                tokio::fs::copy(&bundled_path, &tmp_tarball)
                    .await
                    .map_err(|e| format!("Failed to copy bundled Git: {}", e))?;
            } else {
                let url = format!("https://malalongxia.com/downloads/{}", filename);
                emit_progress(window, channel, 2, "Downloading Git for macOS...");
                download_with_progress(window, channel, &url, &tmp_str, 2, 8).await?;
            }

            emit_progress(window, channel, 9, "Installing Git...");

            // Extract Homebrew bottle to ~/.local/git
            let git_dir = dirs::home_dir()
                .ok_or("Cannot determine home directory")?
                .join(".local")
                .join("git");

            tokio::fs::create_dir_all(&git_dir)
                .await
                .map_err(|e| format!("Failed to create git directory: {}", e))?;

            emit_log(window, channel, &format!("Extracting Git to {} ...", git_dir.display()));

            // Homebrew bottles have a nested structure: git/VERSION/bin/git
            // Extract and flatten with --strip-components=2
            let child = Command::new("tar")
                .args([
                    "-xzf", &tmp_str,
                    "-C", &git_dir.to_string_lossy().to_string(),
                    "--strip-components=2",
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to extract Git: {}", e))?;

            stream_child_output(window, channel, child).await?;

            // Clear macOS Gatekeeper quarantine attribute so binaries can execute
            let _ = Command::new("xattr")
                .args(["-cr", &git_dir.to_string_lossy().to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output()
                .await;
            emit_log(window, channel, "Cleared quarantine attributes.");

            // Add git bin to PATH in shell profile
            let git_bin = git_dir.join("bin");
            let path_export = format!(
                "\n# Git (installed by OpenClaw)\nexport PATH=\"{}:$PATH\"\n",
                git_bin.display()
            );

            let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
            let profile_candidates = [".zshrc", ".zprofile", ".bashrc", ".bash_profile", ".profile"];
            for profile_name in &profile_candidates {
                let profile_path = home.join(profile_name);
                if profile_path.exists() {
                    let content = tokio::fs::read_to_string(&profile_path)
                        .await
                        .unwrap_or_default();
                    if !content.contains(&git_bin.to_string_lossy().to_string()) {
                        let tmp_profile = profile_path.with_extension("tmp");
                        let new_content = format!("{}{}", content, path_export);
                        tokio::fs::write(&tmp_profile, &new_content)
                            .await
                            .map_err(|e| format!("Failed to write temp file: {}", e))?;
                        tokio::fs::rename(&tmp_profile, &profile_path)
                            .await
                            .map_err(|e| format!("Failed to update {}: {}", profile_name, e))?;
                        emit_log(window, channel, &format!("Added Git to PATH in {}", profile_name));
                    }
                    break;
                }
            }

            // Also add to current process PATH
            let current_path = std::env::var("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{}:{}", git_bin.display(), current_path));

            // Cleanup temp file
            let _ = tokio::fs::remove_file(&tmp_tarball).await;

            // Verify git works
            refresh_system_path();
            let git_ok = Command::new("git")
                .arg("--version")
                .env("PATH", expanded_path())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if git_ok {
                emit_log(window, channel, "Git installed successfully.");
                Ok(())
            } else {
                // Last resort: try xcode-select --install
                emit_log(window, channel, "Standalone Git install failed, falling back to Xcode CLT...");
                let child = Command::new("xcode-select")
                    .arg("--install")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("Failed to trigger Xcode CLT install: {}", e))?;
                let _ = stream_child_output(window, channel, child).await;

                Err("Git installation requires manual approval. Please complete the Xcode Command Line Tools installation dialog and retry.".to_string())
            }
        }
        "linux" => {
            Err("Git is not installed. Please install Git using your package manager (e.g. 'sudo apt install git') and try again.".to_string())
        }
        _ => {
            Err(format!("Git is not available on this platform ({}). Please install Git manually.", os))
        }
    }
}

// Build the Node.js download URL from a mirror base URL.
// Mirror URL format: https://npmmirror.com/mirrors/node/
// Result: https://npmmirror.com/mirrors/node/v22.14.0/node-v22.14.0-darwin-arm64.tar.gz
fn build_node_download_url(mirror_base: &str) -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let suffix = match os {
        "macos" => {
            let arch_str = if arch == "aarch64" { "arm64" } else { "x64" };
            format!("node-{}-darwin-{}.tar.gz", NODE_VERSION, arch_str)
        }
        "linux" => {
            let arch_str = if arch == "aarch64" { "arm64" } else { "x64" };
            format!("node-{}-linux-{}.tar.xz", NODE_VERSION, arch_str)
        }
        "windows" => {
            let arch_str = if arch == "aarch64" { "arm64" } else { "x64" };
            format!("node-{}-win-{}.zip", NODE_VERSION, arch_str)
        }
        _ => format!("node-{}-linux-x64.tar.xz", NODE_VERSION),
    };

    let base = mirror_base.trim_end_matches('/');
    format!("{}/{}/{}", base, NODE_VERSION, suffix)
}

#[tauri::command]
pub async fn install_node(mirror: String, method: String, app: tauri::AppHandle, window: Window) -> Result<(), String> {
    let ch = "node-install";
    emit_progress(&window, ch, 0, "Starting Node.js installation...");

    let os = std::env::consts::OS;

    if method == "nvm" {
        // Install via nvm
        emit_progress(&window, ch, 5, "Checking prerequisites...");

        // Check curl is available
        if which::which("curl").is_err() {
            return Err("curl is not installed. Please install curl first.".to_string());
        }

        emit_progress(&window, ch, 10, "Installing nvm...");
        emit_log(&window, ch, "Installing nvm (Node Version Manager)...");

        let nvm_install_script = match os {
            "macos" | "linux" => {
                // Use gitee mirror for nvm script in China
                "https://gitee.com/mirrors/nvm/raw/master/install.sh"
            }
            _ => return Err(format!("nvm installation is not supported on {}", os)),
        };

        // Download and run nvm install script
        let child = cmd("bash")
            .arg("-c")
            .arg(format!("curl -fsSL {} | bash", nvm_install_script))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start nvm install: {}", e))?;

        stream_child_output(&window, ch, child).await?;
        emit_progress(&window, ch, 50, "nvm installed, installing Node.js v22...");

        // Source nvm and install node LTS using the selected mirror
        let nvm_dir = dirs::home_dir()
            .ok_or("Cannot determine home directory")?
            .join(".nvm");
        let nvm_script = nvm_dir.join("nvm.sh");

        // Use the mirror URL for NVM_NODEJS_ORG_MIRROR (passed as env var, not shell string)
        let mirror_base = mirror.trim_end_matches('/');
        let install_cmd = format!(
            "source {} && nvm install 22 && nvm use 22",
            nvm_script.display(),
        );

        let child = cmd("bash")
            .arg("-c")
            .arg(&install_cmd)
            .env("NVM_NODEJS_ORG_MIRROR", mirror_base)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to install Node.js via nvm: {}", e))?;

        stream_child_output(&window, ch, child).await?;
        emit_progress(&window, ch, 95, "Verifying Node.js...");
        post_install_verify(&window, ch).await?;
        emit_progress(&window, ch, 100, "Node.js installed successfully via nvm!");
    } else {
        // Direct installation with download progress
        emit_progress(&window, ch, 5, "Preparing direct installation...");

        match os {
            "macos" | "linux" => {
                let ext = if os == "linux" { "tar.xz" } else { "tar.gz" };
                let tmp_path = std::env::temp_dir()
                    .join(format!("node-installer.{}", ext))
                    .to_string_lossy()
                    .to_string();

                // Check for bundled Node.js archive first (Full edition)
                if let Some(bundled_path) = bundled_node_archive(&app) {
                    emit_log(&window, ch, "Using bundled Node.js (offline)...");
                    emit_progress(&window, ch, 10, "Copying bundled Node.js...");
                    tokio::fs::copy(&bundled_path, &tmp_path)
                        .await
                        .map_err(|e| format!("Failed to copy bundled Node.js: {}", e))?;
                } else {
                    let download_url = build_node_download_url(&mirror);
                    emit_log(&window, ch, &format!("Node.js download URL: {}", download_url));
                    // Download with progress (5% - 70%)
                    download_with_progress(&window, ch, &download_url, &tmp_path, 5, 70).await?;
                }

                emit_progress(&window, ch, 75, "Extracting Node.js...");

                // Extract to user-local directory (no sudo needed)
                let node_dir = dirs::home_dir()
                    .ok_or("Cannot determine home directory")?
                    .join(".local")
                    .join("node");

                emit_log(&window, ch, &format!("Extracting to {} ...", node_dir.display()));

                // Create target directory
                tokio::fs::create_dir_all(&node_dir)
                    .await
                    .map_err(|e| format!("Failed to create directory: {}", e))?;

                let tar_flags = if os == "linux" { "-xJf" } else { "-xzf" };
                let child = cmd("tar")
                    .args([
                        tar_flags,
                        &tmp_path,
                        "-C",
                        &node_dir.to_string_lossy(),
                        "--strip-components=1",
                    ])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("Failed to extract Node.js: {}", e))?;

                stream_child_output(&window, ch, child).await?;

                // Configure PATH in shell profile
                emit_progress(&window, ch, 90, "Configuring PATH...");
                let node_bin = node_dir.join("bin");
                let path_export = format!(
                    "\n# Node.js (installed by OpenClaw)\nexport PATH=\"{}:$PATH\"\n",
                    node_bin.display()
                );

                let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
                // Append to appropriate shell profile
                let profile_candidates = [".zshrc", ".zprofile", ".bashrc", ".bash_profile", ".profile"];
                for profile_name in &profile_candidates {
                    let profile_path = home.join(profile_name);
                    if profile_path.exists() {
                        // Check if already configured
                        let content = tokio::fs::read_to_string(&profile_path)
                            .await
                            .unwrap_or_default();
                        if !content.contains(&node_bin.to_string_lossy().to_string()) {
                            // Atomic write: write to temp file first, then rename
                            let tmp_profile = profile_path.with_extension("tmp");
                            let new_content = format!("{}{}", content, path_export);
                            tokio::fs::write(&tmp_profile, &new_content)
                                .await
                                .map_err(|e| format!("Failed to write temp file: {}", e))?;
                            tokio::fs::rename(&tmp_profile, &profile_path)
                                .await
                                .map_err(|e| format!("Failed to update {}: {}", profile_name, e))?;
                            emit_log(
                                &window,
                                ch,
                                &format!("Added Node.js to PATH in {}", profile_name),
                            );
                        }
                        break;
                    }
                }

                // Cleanup temp file
                let _ = tokio::fs::remove_file(&tmp_path).await;

                emit_progress(&window, ch, 95, "Verifying Node.js...");
                post_install_verify(&window, ch).await?;
                emit_progress(&window, ch, 100, "Node.js installed successfully!");
            }
            "windows" => {
                let tmp_path = std::env::temp_dir().join("node-installer.zip");
                let tmp_str = tmp_path.to_string_lossy().to_string();

                // Check for bundled Node.js archive first (Full edition)
                if let Some(bundled_path) = bundled_node_archive(&app) {
                    emit_log(&window, ch, "Using bundled Node.js (offline)...");
                    emit_progress(&window, ch, 10, "Copying bundled Node.js...");
                    tokio::fs::copy(&bundled_path, &tmp_path)
                        .await
                        .map_err(|e| format!("Failed to copy bundled Node.js: {}", e))?;
                } else {
                    let download_url = build_node_download_url(&mirror);
                    emit_log(&window, ch, &format!("Node.js download URL: {}", download_url));
                    // Download with progress (5% - 70%)
                    download_with_progress(&window, ch, &download_url, &tmp_str, 5, 70).await?;
                }

                emit_progress(&window, ch, 75, "Extracting Node.js...");
                emit_log(&window, ch, "Extracting portable Node.js (no admin required)...");

                // Extract to %LOCALAPPDATA%\Programs\nodejs
                let local_app_data = std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join("AppData")
                            .join("Local")
                            .to_string_lossy()
                            .to_string()
                    });
                let node_dir = std::path::PathBuf::from(&local_app_data)
                    .join("Programs")
                    .join("nodejs");

                // Clean up existing directory if present
                if node_dir.exists() {
                    let _ = Command::new("cmd")
                        .args(["/C", "rmdir", "/s", "/q", &node_dir.to_string_lossy()])
                        .output()
                        .await;
                }

                let _ = tokio::fs::create_dir_all(&node_dir).await;

                // Extract zip: try tar.exe first, then PowerShell
                let tar_result = Command::new("tar")
                    .args(["-xf", &tmp_str, "-C", &node_dir.to_string_lossy(), "--strip-components=1"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn();

                match tar_result {
                    Ok(child) => {
                        stream_child_output(&window, ch, child).await?;
                    }
                    Err(_) => {
                        // Fallback: PowerShell Expand-Archive + move contents
                        emit_log(&window, ch, "tar not available, trying PowerShell...");
                        let tmp_extract = std::env::temp_dir().join("node-extract");
                        let extract_cmd = format!(
                            "Remove-Item -Recurse -Force '{}' -ErrorAction SilentlyContinue; \
                             Expand-Archive -Force -Path '{}' -DestinationPath '{}'; \
                             $sub = Get-ChildItem '{}' -Directory | Select-Object -First 1; \
                             if ($sub) {{ Get-ChildItem $sub.FullName | Move-Item -Destination '{}' -Force }}; \
                             Remove-Item -Recurse -Force '{}' -ErrorAction SilentlyContinue",
                            tmp_extract.display(), tmp_str, tmp_extract.display(),
                            tmp_extract.display(), node_dir.display(), tmp_extract.display()
                        );
                        let ps_paths = [
                            "powershell.exe".to_string(),
                            format!("{}\\WindowsPowerShell\\v1.0\\powershell.exe",
                                std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())),
                        ];
                        let mut ps_child = None;
                        for ps in &ps_paths {
                            if let Ok(c) = Command::new(ps)
                                .args(["-NoProfile", "-Command", &extract_cmd])
                                .stdout(std::process::Stdio::piped())
                                .stderr(std::process::Stdio::piped())
                                .spawn()
                            {
                                ps_child = Some(c);
                                break;
                            }
                        }
                        let child = ps_child.ok_or_else(|| {
                            "Failed to extract Node.js: neither tar nor PowerShell found".to_string()
                        })?;
                        stream_child_output(&window, ch, child).await?;
                    }
                }

                // Add to current process PATH
                let current_path = std::env::var("PATH").unwrap_or_default();
                std::env::set_var("PATH", format!("{};{}", node_dir.display(), current_path));

                // Also add npm global directory
                let appdata = std::env::var("APPDATA").unwrap_or_default();
                if !appdata.is_empty() {
                    let npm_global = std::path::PathBuf::from(&appdata).join("npm");
                    let _ = tokio::fs::create_dir_all(&npm_global).await;
                    let current_path = std::env::var("PATH").unwrap_or_default();
                    if !current_path.contains(&npm_global.to_string_lossy().to_string()) {
                        std::env::set_var("PATH", format!("{};{}", npm_global.display(), current_path));
                    }
                }

                // Cleanup temp file
                let _ = tokio::fs::remove_file(&tmp_path).await;

                emit_progress(&window, ch, 90, "Verifying Node.js...");
                post_install_verify(&window, ch).await?;
                emit_progress(&window, ch, 100, "Node.js installed successfully!");
            }
            _ => return Err(format!("Unsupported OS: {}", os)),
        }
    }

    Ok(())
}

// Classify npm install errors and provide actionable hints.
fn classify_install_error(err: &str) -> &'static str {
    let lower = err.to_lowercase();
    if lower.contains("eacces") || lower.contains("eperm") || lower.contains("permission denied") {
        "Permission error. Try running the app as Administrator (Windows) or check file permissions."
    } else if lower.contains("enospc") {
        "Disk full. Free up disk space and try again."
    } else if lower.contains("etarget") || lower.contains("enoent") {
        "Package not found. The npm registry may be temporarily unavailable."
    } else if lower.contains("network") || lower.contains("etimedout") || lower.contains("econnrefused") || lower.contains("econnreset") {
        "Network error. Check your internet connection or try a different mirror."
    } else if lower.contains("node-gyp") || lower.contains("cmake") || lower.contains("msbuild") {
        "Native module compilation failed. This is usually not critical for OpenClaw."
    } else if lower.contains("sharp") || lower.contains("libvips") {
        "Image processing module error. This is usually not critical."
    } else {
        ""
    }
}

#[tauri::command]
pub async fn install_openclaw(mirror: String, app: tauri::AppHandle, window: Window) -> Result<InstallResult, String> {
    let ch = "openclaw-install";
    let registry = mirror.trim_end_matches('/');

    // Pre-flight: ensure git is available (some npm packages need it)
    refresh_system_path();
    let git_ok = cmd("git")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !git_ok {
        emit_log(&window, ch, "Git not found. Installing portable Git...");
        install_portable_git(&window, ch, &app).await?;
        refresh_system_path();
    } else {
        emit_log(&window, ch, "Git detected.");
    }

    // Pre-flight: ensure npm is available before proceeding
    let npm_check = cmd("npm")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    match npm_check {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            emit_log(&window, ch, &format!("npm {} detected.", v));
        }
        _ => {
            return Err(
                "npm is not available. Please install Node.js first and ensure it is in your PATH."
                    .to_string(),
            );
        }
    }

    // Step 1: Will pass --registry flag directly to npm install (avoids modifying global config)
    emit_progress(&window, ch, 10, "Preparing installation...");
    emit_log(&window, ch, &format!("Using npm registry: {}", registry));

    // Build git config env vars to avoid modifying user's global gitconfig.
    // GIT_CONFIG_COUNT + GIT_CONFIG_KEY_N + GIT_CONFIG_VALUE_N let us set
    // temporary config for child processes only.
    let mut git_config_entries: Vec<(String, String)> = vec![
        // Rewrite SSH → HTTPS so users without SSH keys aren't blocked
        ("url.https://github.com/.insteadOf".to_string(), "ssh://git@github.com/".to_string()),
        ("url.https://github.com/.insteadOf".to_string(), "git@github.com:".to_string()),
    ];

    // Test direct GitHub connectivity (many Chinese users can't reach it)
    let github_ok = reqwest::Client::new()
        .head("https://github.com")
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().is_redirection())
        .unwrap_or(false);

    if !github_ok {
        emit_log(&window, ch, "GitHub unreachable, configuring mirror proxy...");
        let mirrors = [
            "https://ghfast.top/https://github.com/",
            "https://mirror.ghproxy.com/https://github.com/",
            "https://gh-proxy.com/https://github.com/",
        ];
        for mirror_url in &mirrors {
            let ok = reqwest::Client::new()
                .head(*mirror_url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .is_ok();
            if ok {
                emit_log(&window, ch, &format!("Using GitHub mirror: {}", mirror_url));
                git_config_entries.push((
                    format!("url.{}.insteadOf", mirror_url),
                    "https://github.com/".to_string(),
                ));
                break;
            }
        }
    }

    // Step 2: Install openclaw globally with retry mechanism
    emit_progress(&window, ch, 20, "Installing openclaw...");

    let max_retries = 3;
    let mut last_error = String::new();
    for attempt in 1..=max_retries {
        if attempt > 1 {
            emit_log(&window, ch, &format!("Retry attempt {}/{}...", attempt, max_retries));
            emit_progress(&window, ch, 20, &format!("Retrying installation ({}/{})...", attempt, max_retries));
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        emit_log(&window, ch, "Running: npm install -g openclaw@latest");

        let mut npm_cmd = cmd("npm");
        npm_cmd
            .args(["install", "-g", "openclaw@latest", "--registry", registry])
            .env("SHARP_IGNORE_GLOBAL_LIBVIPS", "1")
            // Skip node-llama-cpp native compilation — most users use API mode,
            // not local models. This avoids cmake/xpm/Vulkan build failures.
            .env("NODE_LLAMA_CPP_SKIP_DOWNLOAD", "true");

        // Pass git config via environment variables (not global gitconfig)
        npm_cmd.env("GIT_CONFIG_COUNT", git_config_entries.len().to_string());
        for (i, (key, value)) in git_config_entries.iter().enumerate() {
            npm_cmd.env(format!("GIT_CONFIG_KEY_{}", i), key);
            npm_cmd.env(format!("GIT_CONFIG_VALUE_{}", i), value);
        }

        let child = npm_cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to install openclaw: {}", e))?;

        match stream_child_output(&window, ch, child).await {
            Ok(()) => {
                last_error.clear();
                break;
            }
            Err(e) => {
                let error_hint = classify_install_error(&e);
                emit_log(&window, ch, &format!("Install attempt {} failed: {}", attempt, e));
                if !error_hint.is_empty() {
                    emit_log(&window, ch, &format!("Hint: {}", error_hint));
                }
                last_error = e;
                // Clean up before retry
                if attempt < max_retries {
                    // Remove leftover openclaw dir to avoid EPERM file lock issues on Windows
                    emit_log(&window, ch, "Cleaning up before retry...");
                    let npm_prefix = cmd("npm")
                        .args(["config", "get", "prefix"])
                        .output()
                        .await
                        .ok()
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_default();
                    if !npm_prefix.is_empty() {
                        let leftover = std::path::PathBuf::from(&npm_prefix)
                            .join("node_modules")
                            .join("openclaw");
                        if leftover.exists() {
                            emit_log(&window, ch, &format!("Removing leftover {}", leftover.display()));
                            // On Windows, use rmdir /s /q for better handling of locked files
                            if cfg!(windows) {
                                let _ = Command::new("cmd")
                                    .args(["/C", "rmdir", "/s", "/q", &leftover.to_string_lossy()])
                                    .output()
                                    .await;
                            } else {
                                let _ = tokio::fs::remove_dir_all(&leftover).await;
                            }
                        }
                    }
                    let _ = cmd("npm").args(["cache", "clean", "--force"]).output().await;
                }
            }
        }
    }

    if !last_error.is_empty() {
        // npm may report errors from optional/postinstall scripts even though
        // the core package was installed successfully. Check if openclaw is usable.
        emit_log(&window, ch, "Checking if openclaw is usable despite npm errors...");
        refresh_system_path();
        let oc_check = cmd("openclaw")
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;
        match oc_check {
            Ok(o) if o.status.success() => {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                emit_log(&window, ch, &format!("openclaw {} is usable despite npm warnings.", v));
                // Continue — installation is partially successful but functional
            }
            _ => {
                return Err(format!(
                    "Failed to install openclaw after {} attempts: {}",
                    max_retries, last_error
                ));
            }
        }
    }

    emit_progress(&window, ch, 100, "OpenClaw installed successfully!");

    // Step 3: Retrieve installed version
    let version_output = cmd("npm")
        .args(["list", "-g", "openclaw", "--depth=0", "--json"])
        .output()
        .await
        .map_err(|e| format!("Failed to query openclaw version: {}", e))?;

    let version = if version_output.status.success() {
        let json_str = String::from_utf8_lossy(&version_output.stdout);
        serde_json::from_str::<serde_json::Value>(&json_str)
            .ok()
            .and_then(|v| {
                v.get("dependencies")
                    .and_then(|d| d.get("openclaw"))
                    .and_then(|o| o.get("version"))
                    .and_then(|ver| ver.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    Ok(InstallResult { version })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_with_trailing_slash() {
        let url = build_node_download_url("https://npmmirror.com/mirrors/node/");
        assert!(url.starts_with("https://npmmirror.com/mirrors/node/v22.14.0/"));
        assert!(url.contains("node-v22.14.0"));
    }

    #[test]
    fn build_url_without_trailing_slash() {
        let url = build_node_download_url("https://npmmirror.com/mirrors/node");
        assert!(url.starts_with("https://npmmirror.com/mirrors/node/v22.14.0/"));
    }

    #[test]
    fn build_url_contains_correct_extension() {
        let url = build_node_download_url("https://example.com");
        let os = std::env::consts::OS;
        match os {
            "macos" => assert!(url.ends_with(".tar.gz")),
            "linux" => assert!(url.ends_with(".tar.xz")),
            "windows" => assert!(url.ends_with(".zip")),
            _ => assert!(url.ends_with(".tar.xz")),
        }
    }

    #[test]
    fn build_url_contains_correct_arch() {
        let url = build_node_download_url("https://example.com");
        let arch = std::env::consts::ARCH;
        if arch == "aarch64" {
            assert!(url.contains("arm64"));
        } else {
            assert!(url.contains("x64"));
        }
    }

    #[test]
    fn build_url_includes_node_version() {
        let url = build_node_download_url("https://example.com");
        assert!(url.contains(NODE_VERSION));
    }

    // Platform-specific URL format validation
    #[test]
    fn build_url_macos_format() {
        let url = build_node_download_url("https://mirror.example.com");
        let os = std::env::consts::OS;
        if os == "macos" {
            assert!(url.contains("darwin"), "macOS URL should contain 'darwin': {}", url);
            assert!(url.ends_with(".tar.gz"), "macOS should use .tar.gz: {}", url);
            assert!(!url.contains("linux"), "macOS URL should not contain 'linux'");
        }
    }

    #[test]
    fn build_url_linux_format() {
        let url = build_node_download_url("https://mirror.example.com");
        let os = std::env::consts::OS;
        if os == "linux" {
            assert!(url.contains("linux"), "Linux URL should contain 'linux': {}", url);
            assert!(url.ends_with(".tar.xz"), "Linux should use .tar.xz: {}", url);
            assert!(!url.contains("darwin"), "Linux URL should not contain 'darwin'");
        }
    }

    #[test]
    fn build_url_windows_format() {
        let url = build_node_download_url("https://mirror.example.com");
        let os = std::env::consts::OS;
        if os == "windows" {
            assert!(url.ends_with(".zip"), "Windows should use .zip: {}", url);
            assert!(!url.contains("darwin"), "Windows URL should not contain 'darwin'");
            assert!(!url.contains("linux"), "Windows URL should not contain 'linux'");
        }
    }

    // URL structure validation
    #[test]
    fn build_url_format_is_base_version_filename() {
        let url = build_node_download_url("https://example.com/mirror");
        // Should be: base/NODE_VERSION/node-NODE_VERSION-os-arch.ext
        let parts: Vec<&str> = url.splitn(2, NODE_VERSION).collect();
        assert_eq!(parts.len(), 2, "URL should contain version exactly: {}", url);
        assert!(parts[0].ends_with('/'), "Version should be preceded by /: {}", url);
        assert!(parts[1].starts_with('/'), "Version should be followed by /: {}", url);
    }

    #[test]
    fn build_url_strips_multiple_trailing_slashes() {
        let url = build_node_download_url("https://example.com///");
        assert!(!url.contains("///"), "Should strip trailing slashes: {}", url);
        assert!(url.starts_with("https://example.com/"));
    }

    // NODE_VERSION constant validation
    #[test]
    fn node_version_starts_with_v() {
        assert!(NODE_VERSION.starts_with('v'), "NODE_VERSION should start with 'v'");
    }

    #[test]
    fn node_version_is_semver() {
        let stripped = &NODE_VERSION[1..]; // Remove 'v'
        let parts: Vec<&str> = stripped.split('.').collect();
        assert_eq!(parts.len(), 3, "Version should be semver: {}", NODE_VERSION);
        for part in &parts {
            assert!(part.parse::<u32>().is_ok(), "Version part not numeric: {}", part);
        }
    }

    // InstallResult struct
    #[test]
    fn install_result_serializes() {
        let result = InstallResult { version: "1.2.3".to_string() };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"version\":\"1.2.3\""));
    }

    // Temp dir usage (cross-platform)
    #[test]
    fn temp_dir_exists() {
        let tmp = std::env::temp_dir();
        assert!(tmp.exists(), "Temp directory should exist: {:?}", tmp);
    }

    // Shell profile candidates exist on Unix-like systems
    #[test]
    fn shell_profile_candidates_are_valid() {
        let candidates = [".zshrc", ".bashrc", ".profile"];
        for name in &candidates {
            assert!(name.starts_with('.'), "Profile should be a dotfile: {}", name);
        }
    }
}
