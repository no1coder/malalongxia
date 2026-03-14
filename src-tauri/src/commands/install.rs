use serde::Serialize;
use tauri::{Emitter, Manager, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::path_env::{expanded_path, refresh_system_path};

const NODE_VERSION: &str = "v22.22.0";

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

    // Reject HTML error pages masquerading as downloads
    if let Some(ct) = response.headers().get("content-type") {
        let ct_str = ct.to_str().unwrap_or("");
        if ct_str.starts_with("text/html") {
            return Err(format!(
                "Server returned HTML instead of a file (content-type: {}). The download URL may be invalid.",
                ct_str
            ));
        }
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

    // Reject suspiciously small files (< 1 MB) — these are almost certainly
    // error pages or truncated transfers, not valid Node.js / MinGit archives.
    if downloaded < 1_048_576 {
        return Err(format!(
            "Download produced only {} bytes (expected at least 1 MB). \
             The server may have returned an error page or the transfer was truncated.",
            downloaded
        ));
    }

    let downloaded_mb = downloaded as f64 / 1_048_576.0;
    emit_log(window, channel, &format!("Download complete: {:.1}MB", downloaded_mb));

    Ok(())
}

// Stream stdout and stderr of a child process to the frontend via events.
// Stderr is also captured so callers can inspect error details on failure
// (e.g. classify ENOTEMPTY / EEXIST / ETIMEDOUT from npm install).
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
    // Capture stderr content (up to 8KB) for error classification while
    // still streaming each line to the frontend in real time.
    let stderr_handle: tokio::task::JoinHandle<String> = tokio::spawn(async move {
        let mut captured = String::new();
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                emit_log(&w2, &ch2, &line);
                if captured.len() < 8192 {
                    captured.push_str(&line);
                    captured.push('\n');
                }
            }
        }
        captured
    });

    // 10-minute timeout covers stdout/stderr drain AND process exit.
    // Without this, hung child processes would block stdout/stderr join forever.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(600),
        async {
            let (_, stderr_res) = tokio::join!(stdout_handle, stderr_handle);
            let captured = stderr_res.unwrap_or_else(|_| String::new());
            let wait_res = child.wait().await;
            (captured, wait_res)
        },
    )
    .await;

    match result {
        Err(_) => {
            // Timeout elapsed. On Windows, kill the entire process tree (cmd.exe → npm → node
            // → postinstall scripts) using `taskkill /T /F /PID`. A plain Child::kill() only
            // terminates the direct process (cmd.exe), leaving npm/node as orphans that hold
            // port 18789 or npm lock files. On Unix, kill the process group.
            // Use tokio::process::Command (async) to avoid blocking the tokio executor.
            #[cfg(windows)]
            if let Some(pid) = child.id() {
                let _ = Command::new("taskkill")
                    .args(["/T", "/F", "/PID", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await;
            }
            #[cfg(not(windows))]
            let _ = child.kill().await;
            Err("Process timed out after 10 minutes".to_string())
        }
        Ok((captured_stderr, wait_result)) => {
            let status = wait_result.map_err(|e| format!("Failed to wait for process: {}", e))?;
            if !status.success() {
                // Include the last ~2000 chars of stderr in the error message so callers
                // (e.g. classify_install_error) can inspect the actual error details.
                let tail = if captured_stderr.len() > 2000 {
                    &captured_stderr[captured_stderr.len() - 2000..]
                } else {
                    &captured_stderr
                };
                if tail.trim().is_empty() {
                    return Err(format!("Process exited with status: {}", status));
                }
                return Err(format!("Process exited with status: {}. Output: {}", status, tail.trim()));
            }
            Ok(())
        }
    }
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

            // Clean up any previous (possibly corrupt) MinGit installation before extracting.
            // Retry up to 3 times to handle Windows Defender file locks.
            if git_dir.exists() {
                let dir_str = git_dir.to_string_lossy().to_string();
                for attempt in 1u8..=3 {
                    let ok = Command::new("cmd")
                        .args(["/C", "rmdir", "/s", "/q", &dir_str])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .await
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if ok || !git_dir.exists() { break; }
                    if attempt < 3 {
                        emit_log(window, channel, &format!(
                            "MinGit dir cleanup attempt {}/3 failed (file lock?), retrying...", attempt
                        ));
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    }
                }
            }

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
                    // Escape single quotes for PowerShell string literals (e.g. O'Brien paths)
                    let ps_tmp = tmp_str.replace('\'', "''");
                    let ps_dest = git_dir.display().to_string().replace('\'', "''");
                    let extract_cmd = format!(
                        "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
                        ps_tmp,
                        ps_dest
                    );
                    let ps_paths = [
                        "powershell.exe".to_string(),
                        format!("{}\\WindowsPowerShell\\v1.0\\powershell.exe",
                            std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())),
                    ];
                    let mut ps_child = None;
                    for ps in &ps_paths {
                        if let Ok(c) = Command::new(ps)
                            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &extract_cmd])
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

            // Verify git.exe was actually extracted before updating PATH.
            // Retry up to 5 times with short delays — Windows Defender may still
            // be scanning the newly extracted files, causing transient file-not-found.
            let git_exe = git_dir.join("cmd").join("git.exe");
            let mut git_found = git_exe.exists();
            for _ in 0..4 {
                if git_found { break; }
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                git_found = git_exe.exists();
            }
            if !git_found {
                return Err(format!(
                    "MinGit extraction failed: git.exe not found in {}. \
                     The archive may be corrupt — please retry.",
                    git_dir.display()
                ));
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

            // Clean up any previous (possibly corrupt) installation before extracting.
            if git_dir.exists() {
                emit_log(window, channel, "Removing previous Git installation...");
                let _ = tokio::fs::remove_dir_all(&git_dir).await;
            }

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
// Result: https://npmmirror.com/mirrors/node/v22.22.0/node-v22.22.0-darwin-arm64.tar.gz
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

        if !matches!(os, "macos" | "linux") {
            return Err(format!("nvm installation is not supported on {}", os));
        }

        // Try nvm install script sources in order.
        // Gitee mirror is preferred for Chinese users; raw.githubusercontent.com
        // is the official source and serves as a fallback when Gitee is unavailable.
        let nvm_script_sources = [
            "https://gitee.com/mirrors/nvm/raw/master/install.sh",
            "https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh",
        ];

        let mut nvm_installed = false;
        for nvm_script_url in &nvm_script_sources {
            emit_log(&window, ch, &format!("Trying nvm install script: {}", nvm_script_url));
            let child = cmd("bash")
                .arg("-c")
                .arg(format!("curl -fsSL {} | bash", nvm_script_url))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start nvm install: {}", e))?;

            match stream_child_output(&window, ch, child).await {
                Ok(()) => { nvm_installed = true; break; }
                Err(e) => {
                    emit_log(&window, ch, &format!("nvm script {} failed: {}", nvm_script_url, e));
                }
            }
        }
        if !nvm_installed {
            return Err("Failed to download nvm install script from all sources.".to_string());
        }
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

        // Add nvm node bin to process PATH so subsequent commands find node/npm/openclaw.
        // The child shell ran `nvm use 22` but that only affects the child's env.
        let nvm_node_bin = nvm_dir.join("versions").join("node");
        if let Ok(entries) = glob::glob(&nvm_node_bin.join("*/bin").to_string_lossy()) {
            let mut matches: Vec<std::path::PathBuf> = entries.filter_map(|e| e.ok()).collect();
            matches.sort_by(|a, b| {
                super::path_env::version_tuple_from_path(a)
                    .cmp(&super::path_env::version_tuple_from_path(b))
            });
            if let Some(latest_bin) = matches.pop() {
                let current = std::env::var("PATH").unwrap_or_default();
                if !current.contains(&latest_bin.to_string_lossy().to_string()) {
                    std::env::set_var("PATH", format!("{}:{}", latest_bin.display(), current));
                    emit_log(&window, ch, &format!("Added {} to PATH", latest_bin.display()));
                }
            }
        }

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

                // Clean up existing directory if present.
                // Retry up to 3 times with a short delay — Windows Defender / antivirus
                // file scanning can briefly lock files and cause rmdir to fail.
                if node_dir.exists() {
                    let dir_str = node_dir.to_string_lossy().to_string();
                    let mut removed = false;
                    for attempt in 1u8..=3 {
                        let status = Command::new("cmd")
                            .args(["/C", "rmdir", "/s", "/q", &dir_str])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status()
                            .await;
                        if status.map(|s| s.success()).unwrap_or(false) || !node_dir.exists() {
                            removed = true;
                            break;
                        }
                        emit_log(&window, ch, &format!(
                            "Directory cleanup attempt {}/3 failed (file lock?), retrying...", attempt
                        ));
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    }
                    if !removed && node_dir.exists() {
                        emit_log(&window, ch, "Warning: could not fully remove old nodejs dir; proceeding anyway.");
                    }
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
                        // Escape single quotes for PowerShell string literals
                        let ps_tmp = tmp_str.replace('\'', "''");
                        let ps_extract = tmp_extract.display().to_string().replace('\'', "''");
                        let ps_node = node_dir.display().to_string().replace('\'', "''");
                        let extract_cmd = format!(
                            "Remove-Item -Recurse -Force '{}' -ErrorAction SilentlyContinue; \
                             Expand-Archive -Force -Path '{}' -DestinationPath '{}'; \
                             $sub = Get-ChildItem '{}' -Directory | Select-Object -First 1; \
                             if ($sub) {{ Get-ChildItem $sub.FullName | Move-Item -Destination '{}' -Force }}; \
                             Remove-Item -Recurse -Force '{}' -ErrorAction SilentlyContinue",
                            ps_extract, ps_tmp, ps_extract,
                            ps_extract, ps_node, ps_extract
                        );
                        let ps_paths = [
                            "powershell.exe".to_string(),
                            format!("{}\\WindowsPowerShell\\v1.0\\powershell.exe",
                                std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())),
                        ];
                        let mut ps_child = None;
                        for ps in &ps_paths {
                            if let Ok(c) = Command::new(ps)
                                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &extract_cmd])
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

                // Verify that node.exe was actually extracted before updating PATH.
                // tar and PowerShell can both exit 0 while producing no output (e.g. bad zip).
                // Retry up to 5 times — Windows Defender may still be scanning extracted files.
                let node_exe = node_dir.join("node.exe");
                let mut node_found = node_exe.exists();
                for _ in 0..4 {
                    if node_found { break; }
                    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                    node_found = node_exe.exists();
                }
                if !node_found {
                    return Err(format!(
                        "Node.js extraction failed: node.exe not found in {}. \
                         The archive may be corrupt — please retry.",
                        node_dir.display()
                    ));
                }

                // Add to current process PATH
                let current_path = std::env::var("PATH").unwrap_or_default();
                std::env::set_var("PATH", format!("{};{}", node_dir.display(), current_path));

                // Persist node_dir and npm global bin to user PATH in Windows registry in one call
                // so it survives app restart. Batch into a single append to avoid TOCTOU races.
                let node_dir_str = node_dir.to_string_lossy().to_string();
                let appdata = std::env::var("APPDATA").unwrap_or_default();
                let npm_global_opt = if !appdata.is_empty() {
                    let npm_global = std::path::PathBuf::from(&appdata).join("npm");
                    let _ = tokio::fs::create_dir_all(&npm_global).await;
                    let current_path = std::env::var("PATH").unwrap_or_default();
                    if !current_path.contains(&npm_global.to_string_lossy().to_string()) {
                        std::env::set_var("PATH", format!("{};{}", npm_global.display(), current_path));
                    }
                    Some(npm_global.to_string_lossy().to_string())
                } else {
                    None
                };
                // Append both directories in a single registry write to avoid race conditions
                let dirs_to_persist: Vec<&str> = {
                    let mut v: Vec<&str> = vec![&node_dir_str];
                    if let Some(ref npm_str) = npm_global_opt {
                        v.push(npm_str);
                    }
                    v
                };
                append_dirs_to_user_path_registry(&dirs_to_persist);

                // Cleanup temp file
                let _ = tokio::fs::remove_file(&tmp_path).await;

                // Invalidate npm prefix cache so the next expanded_path() call
                // discovers the newly installed npm global bin directory.
                super::path_env::invalidate_npm_prefix_cache();

                emit_progress(&window, ch, 90, "Verifying Node.js...");
                post_install_verify(&window, ch).await?;
                emit_progress(&window, ch, 100, "Node.js installed successfully!");
            }
            _ => return Err(format!("Unsupported OS: {}", os)),
        }
    }

    Ok(())
}

/// Clean up stale openclaw staging directories under the npm global root.
/// npm creates temporary `.openclaw-*` directories during `npm install -g openclaw`
/// that can be left behind after a failed install. The official install.sh performs
/// the same cleanup: `rm -rf "$npm_root"/.openclaw-* "$npm_root"/openclaw`
async fn cleanup_npm_openclaw_dirs(npm_root: &std::path::Path) {
    if !npm_root.exists() {
        return;
    }
    if let Ok(mut entries) = tokio::fs::read_dir(npm_root).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Remove .openclaw-* staging directories (e.g. .openclaw-a1b2c3)
            if name_str.starts_with(".openclaw-") || name_str.starts_with(".openclaw~") {
                let path = entry.path();
                if path.is_dir() {
                    let _ = tokio::fs::remove_dir_all(&path).await;
                } else {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
        }
    }
}

// Classify npm install errors and provide actionable hints.
fn classify_install_error(err: &str) -> &'static str {
    let lower = err.to_lowercase();
    if lower.contains("enotempty") {
        "ENOTEMPTY: stale staging directory from a previous install. The installer will clean up and retry."
    } else if lower.contains("eexist") {
        "EEXIST: file conflict from a previous install. The installer will remove conflicting files and retry."
    } else if lower.contains("eacces") || lower.contains("eperm") || lower.contains("permission denied") {
        "Permission error. Try running the app as Administrator (Windows) or check file permissions."
    } else if lower.contains("enospc") {
        "Disk full. Free up disk space and try again."
    } else if lower.contains("etarget") || lower.contains("enoent") {
        "Package not found. The npm registry may be temporarily unavailable."
    } else if lower.contains("ssh") || lower.contains("git connection error") || lower.contains("ls-remote") || lower.contains("could not read from remote") {
        "Git SSH connection failed (port 22 blocked). This is a known issue in China — the installer will retry with HTTPS rewriting enabled."
    } else if lower.contains("codeload.github.com") || (lower.contains("etimedout") && lower.contains("github")) {
        "GitHub tarball download timed out (codeload.github.com unreachable). Retrying with --omit=optional to skip optional GitHub-hosted dependencies."
    } else if lower.contains("getaddrinfo") || lower.contains("enotfound") || (lower.contains("dns") && lower.contains("fail")) {
        "DNS resolution failed. Try changing your DNS to 119.29.29.29 (DNSPod) or 223.5.5.5 (Alibaba) and retry."
    } else if lower.contains("network") || lower.contains("etimedout") || lower.contains("econnrefused") || lower.contains("econnreset") {
        "Network error. Check your internet connection or try a different mirror."
    } else if lower.contains("gyp err") || lower.contains("node-gyp") || lower.contains("no developer tools") || lower.contains("failed to build") {
        "Native module compilation failed. Install build tools (macOS: xcode-select --install, Linux: sudo apt install build-essential, Windows: npm install -g windows-build-tools) if needed. This is usually not critical for core OpenClaw functionality."
    } else if lower.contains("cmake") || lower.contains("msbuild") {
        "Build tool (cmake/msbuild) not found. This is usually not critical for OpenClaw."
    } else if lower.contains("sharp") || lower.contains("libvips") {
        "Image processing module error. This is usually not critical."
    } else if lower.contains("python") && (lower.contains("not found") || lower.contains("no python")) {
        "Python not found. Some optional native modules require Python. Install Python 3 if needed."
    } else {
        ""
    }
}

/// Read the current user PATH value from the Windows registry (HKCU\Environment).
/// Returns the raw value string, or an empty string if not set.
#[cfg(windows)]
fn read_user_path_from_registry() -> String {
    use std::process::Command as StdCommand;
    StdCommand::new("reg")
        .args(["query", r"HKCU\Environment", "/v", "Path"])
        .output()
        .ok()
        .and_then(|o| {
            let out = String::from_utf8_lossy(&o.stdout).to_string();
            out.lines()
                .find(|l| l.contains("REG_"))
                .and_then(|line| {
                    if let Some(pos) = line.find("REG_EXPAND_SZ") {
                        Some(line[pos + "REG_EXPAND_SZ".len()..].trim().to_string())
                    } else if let Some(pos) = line.find("REG_SZ") {
                        Some(line[pos + "REG_SZ".len()..].trim().to_string())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_default()
}

/// Append multiple directories to the current user's PATH in the Windows registry
/// (HKCU\Environment) in a single read-modify-write cycle. This avoids TOCTOU races
/// that would occur if each directory were appended in a separate call.
/// Deduplicates case-insensitively. Uses REG_EXPAND_SZ to preserve %VAR% references.
#[cfg(windows)]
fn append_dirs_to_user_path_registry(new_dirs: &[&str]) {
    use std::process::Command as StdCommand;

    let existing = read_user_path_from_registry();
    let lower_existing = existing.to_lowercase();

    // Collect dirs that are not already present
    let mut to_add: Vec<&str> = new_dirs
        .iter()
        .filter(|&&d| {
            let lower_d = d.to_lowercase();
            !lower_existing.split(';').any(|e| e.trim() == lower_d.as_str())
        })
        .copied()
        .collect();

    if to_add.is_empty() {
        return; // All directories already in PATH
    }

    let new_path = if existing.is_empty() {
        to_add.join(";")
    } else {
        let mut parts = vec![existing.as_str()];
        parts.append(&mut to_add);
        parts.join(";")
    };

    // Write back using REG_EXPAND_SZ to preserve any existing %VAR% references
    let _ = StdCommand::new("reg")
        .args([
            "add",
            r"HKCU\Environment",
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
            &new_path,
            "/f",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    // Notify the system that environment variables have changed so Explorer and
    // other processes pick up the new PATH without requiring a logoff.
    // SendMessageTimeout(HWND_BROADCAST=0xFFFF, WM_SETTINGCHANGE=0x001A, 0, "Environment")
    // NOTE: Do NOT call SetEnvironmentVariable with $null here — that would delete the PATH.
    let _ = StdCommand::new("powershell")
        .args([
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
            "Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;\
             public class Win32{[DllImport(\"user32.dll\",SetLastError=true)]public static extern \
             IntPtr SendMessageTimeout(IntPtr h,uint m,UIntPtr w,string l,uint f,uint t,out IntPtr r);}'; \
             $r=[IntPtr]::Zero; \
             [Win32]::SendMessageTimeout([IntPtr]0xffff,0x001a,[UIntPtr]::Zero,'Environment',2,5000,[ref]$r) | Out-Null"
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok(); // fire-and-forget; failure is non-fatal
}

#[cfg(not(windows))]
fn append_dirs_to_user_path_registry(_new_dirs: &[&str]) {
    // No-op on non-Windows platforms
}

// Returns true if the error is caused by a GitHub tarball download failure
// (e.g. codeload.github.com ETIMEDOUT). In this case retrying with
// --omit=optional is the best fallback since libsignal-node and similar
// deps are declared as optional or devDependencies in the upstream package.
fn is_github_tarball_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("codeload.github.com")
        || (lower.contains("etimedout") && lower.contains("github"))
        || (lower.contains("network request") && lower.contains("github"))
}

/// Query the installed openclaw version via npm.
async fn get_openclaw_version() -> String {
    let version_output = cmd("npm")
        .args(["list", "-g", "openclaw", "--depth=0", "--json"])
        .output()
        .await
        .ok();

    version_output
        .filter(|o| o.status.success())
        .and_then(|o| {
            let json_str = String::from_utf8_lossy(&o.stdout);
            serde_json::from_str::<serde_json::Value>(&json_str)
                .ok()
                .and_then(|v| {
                    v.get("dependencies")
                        .and_then(|d| d.get("openclaw"))
                        .and_then(|o| o.get("version"))
                        .and_then(|ver| ver.as_str().map(|s| s.to_string()))
                })
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Fix npm global install permissions on Linux.
/// Many Linux users have npm global prefix pointing to /usr which requires sudo.
/// Redirect to ~/.npm-global so npm install -g works without root.
/// Same strategy as the official install.sh fix_npm_permissions().
#[cfg(target_os = "linux")]
async fn fix_npm_permissions_linux(window: &Window, ch: &str) {
    let npm_prefix = cmd("npm")
        .args(["config", "get", "prefix"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if npm_prefix.is_empty() {
        return;
    }

    // Check if the prefix is writable
    let prefix_path = std::path::PathBuf::from(&npm_prefix);
    let lib_path = prefix_path.join("lib");
    let writable = prefix_path.metadata().map(|_| {
        std::fs::metadata(&lib_path)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
            || std::fs::metadata(&prefix_path)
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false)
    }).unwrap_or(false);

    // Also check by attempting to create a test file
    let test_writable = if !writable {
        let test_file = prefix_path.join(".malalongxia_write_test");
        let can_write = tokio::fs::write(&test_file, "test").await.is_ok();
        let _ = tokio::fs::remove_file(&test_file).await;
        can_write
    } else {
        true
    };

    if test_writable {
        return; // npm prefix is writable, no fix needed
    }

    emit_log(window, ch, "npm global prefix is not writable. Configuring user-local prefix (~/.npm-global)...");

    let home = dirs::home_dir().unwrap_or_default();
    let npm_global = home.join(".npm-global");
    let _ = tokio::fs::create_dir_all(&npm_global).await;

    // Set npm prefix to user-writable directory
    let _ = cmd("npm")
        .args(["config", "set", "prefix", &npm_global.to_string_lossy()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await;

    // Add to PATH for this session
    let npm_global_bin = npm_global.join("bin");
    let current_path = std::env::var("PATH").unwrap_or_default();
    if !current_path.contains(&npm_global_bin.to_string_lossy().to_string()) {
        std::env::set_var("PATH", format!("{}:{}", npm_global_bin.display(), current_path));
    }

    // Persist to shell profiles
    let path_line = format!(
        "\n# npm global (configured by OpenClaw installer)\nexport PATH=\"{}:$PATH\"\n",
        npm_global_bin.display()
    );
    for rc_name in &[".bashrc", ".zshrc"] {
        let rc_path = home.join(rc_name);
        if rc_path.exists() {
            let content = tokio::fs::read_to_string(&rc_path).await.unwrap_or_default();
            if !content.contains(".npm-global") {
                let tmp = rc_path.with_extension("tmp");
                let new_content = format!("{}{}", content, path_line);
                if tokio::fs::write(&tmp, &new_content).await.is_ok() {
                    let _ = tokio::fs::rename(&tmp, &rc_path).await;
                    emit_log(window, ch, &format!("Added ~/.npm-global/bin to PATH in {}", rc_name));
                }
            }
        }
    }

    emit_log(window, ch, "npm global prefix configured to ~/.npm-global (no sudo needed).");
}

#[tauri::command]
pub async fn install_openclaw(mirror: String, app: tauri::AppHandle, window: Window) -> Result<InstallResult, String> {
    let ch = "openclaw-install";
    let registry = mirror.trim_end_matches('/');

    emit_progress(&window, ch, 0, "Starting OpenClaw installation...");

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

    // Linux: fix npm global permissions to avoid EACCES errors.
    // Redirect npm global prefix to ~/.npm-global (same strategy as official install.sh).
    #[cfg(target_os = "linux")]
    {
        fix_npm_permissions_linux(&window, ch).await;
    }

    // Step 1: Will pass --registry flag directly to npm install (avoids modifying global config)
    emit_progress(&window, ch, 10, "Preparing installation...");
    emit_log(&window, ch, &format!("Using npm registry: {}", registry));

    // Build git config entries to rewrite SSH → HTTPS so users without SSH
    // keys (or with port 22 blocked) aren't blocked by git deps like libsignal-node.
    // NOTE: GIT_CONFIG_COUNT/KEY/VALUE requires Git 2.31+. We also write a
    // temp gitconfig file and set GIT_CONFIG_GLOBAL so older Git versions work.
    //
    // Each GIT_CONFIG_COUNT slot holds exactly one key=value pair.
    // Both SSH-URL and SCP-style rewrites use `insteadOf` (there is no `insteadOfScp` key).
    let mut git_config_entries: Vec<(String, String)> = vec![
        // Rewrite ssh://git@github.com/ → https://github.com/
        ("url.https://github.com/.insteadOf".to_string(), "ssh://git@github.com/".to_string()),
        // Rewrite git@github.com: → https://github.com/ (SCP-style shorthand)
        ("url.https://github.com/.insteadOf".to_string(), "git@github.com:".to_string()),
    ];

    // Test direct GitHub connectivity (many Chinese users can't reach it).
    // Use a generous 15s timeout — GFW may cause slow-but-eventual responses.
    let github_ok = reqwest::Client::new()
        .head("https://github.com")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().is_redirection())
        .unwrap_or(false);

    if !github_ok {
        emit_log(&window, ch, "GitHub unreachable, configuring mirror proxy...");
        // Ordered by reliability for Chinese users. github.com is intentionally
        // excluded — it's not a mirror and would just fail again.
        let mirrors = [
            "https://ghfast.top/https://github.com/",
            "https://gh-proxy.com/https://github.com/",
            "https://ghproxy.net/https://github.com/",
            "https://mirror.ghproxy.com/https://github.com/",
            "https://github.moeyy.xyz/https://github.com/",
            "https://hub.gitmirror.com/https://github.com/",
        ];
        for mirror_url in &mirrors {
            let ok = reqwest::Client::new()
                .head(*mirror_url)
                .timeout(std::time::Duration::from_secs(8))
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

    // Snapshot the expanded PATH once before the retry loop so we don't repeatedly
    // invoke `npm config get prefix` (a blocking subprocess) inside each cmd() call.
    let path_snapshot = expanded_path();

    let max_retries = 3;
    let mut last_error = String::new();
    // Track whether a previous attempt hit a GitHub tarball timeout so we can
    // escalate to --omit=optional on the next retry.
    let mut github_tarball_failed = false;
    for attempt in 1..=max_retries {
        if attempt > 1 {
            emit_log(&window, ch, &format!("Retry attempt {}/{}...", attempt, max_retries));
            emit_progress(&window, ch, 20, &format!("Retrying installation ({}/{})...", attempt, max_retries));
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        // Build the base npm install args.
        // --omit=optional is used when GitHub is unreachable (including on the
        // first attempt) because optional deps like libsignal-node reference
        // codeload.github.com tarballs directly — git url rewrites don't help
        // for npm's HTTP tarball fetching. Skipping optional deps avoids the
        // ETIMEDOUT entirely without affecting core OpenClaw functionality.
        let omit_optional = !github_ok || github_tarball_failed;
        let log_suffix = if omit_optional { " --omit=optional --foreground-scripts" } else { " --foreground-scripts" };
        emit_log(&window, ch, &format!("Running: npm install -g openclaw@latest{}", log_suffix));
        if omit_optional {
            emit_log(&window, ch, "Note: --omit=optional skips GitHub-hosted optional dependencies (e.g. libsignal-node) that are unreachable from China.");
        }

        // Write a temporary gitconfig so SSH→HTTPS rewriting works on all Git versions.
        // GIT_CONFIG_COUNT/KEY/VALUE env vars require Git 2.31+; GIT_CONFIG_GLOBAL
        // works on any Git version and takes precedence over the user's global config.
        let tmp_gitconfig = std::env::temp_dir().join("malalongxia_npm_install.gitconfig");
        {
            // The first two entries always target https://github.com/ (SSH rewrites).
            // Any extra entry added later is a mirror rewrite block.
            let mut lines = String::new();
            lines.push_str("[url \"https://github.com/\"]\n");
            lines.push_str(&format!("\tinsteadOf = {}\n", "ssh://git@github.com/"));
            lines.push_str(&format!("\tinsteadOf = {}\n", "git@github.com:"));
            // Append mirror rewrite blocks appended dynamically after connectivity check
            for (key, value) in git_config_entries.iter().skip(2) {
                // key is like "url.MIRROR_URL.insteadOf"
                if let Some(mirror_url) = key.strip_prefix("url.").and_then(|s| s.strip_suffix(".insteadOf")) {
                    lines.push_str(&format!("[url \"{}\"]\n", mirror_url));
                    lines.push_str(&format!("\tinsteadOf = {}\n", value));
                }
            }
            let _ = std::fs::write(&tmp_gitconfig, lines);
        }

        let mut npm_args = vec!["install", "-g", "openclaw@latest", "--registry", registry];
        // --omit=optional skips optional deps that reference GitHub tarballs directly.
        // Used as a fallback when GitHub (codeload.github.com) is not reachable.
        if omit_optional {
            npm_args.push("--omit=optional");
        }
        // --foreground-scripts keeps postinstall output in the current process stdout/stderr
        // instead of spawning a detached console window on Windows, which would cause
        // stream_child_output to time out waiting for a child that never closes its pipes.
        npm_args.push("--foreground-scripts");

        // Use the pre-snapshotted PATH to avoid calling expanded_path() (which may
        // run `npm config get prefix`) on every retry iteration.
        #[cfg(windows)]
        let mut npm_cmd = {
            let mut c = Command::new("cmd");
            c.args(["/C", "npm"]);
            c.env("PATH", &path_snapshot);
            c
        };
        #[cfg(not(windows))]
        let mut npm_cmd = {
            let mut c = Command::new("npm");
            c.env("PATH", &path_snapshot);
            c
        };
        npm_cmd
            .args(&npm_args)
            .env("SHARP_IGNORE_GLOBAL_LIBVIPS", "1")
            // Skip node-llama-cpp native compilation — most users use API mode,
            // not local models. This avoids cmake/xpm/Vulkan build failures.
            .env("NODE_LLAMA_CPP_SKIP_DOWNLOAD", "true")
            // Point Git at our temp config (works on all Git versions)
            .env("GIT_CONFIG_GLOBAL", &tmp_gitconfig);

        // Also pass via GIT_CONFIG_COUNT/KEY/VALUE for Git 2.31+ (belt-and-suspenders)
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
                // Remember if this attempt hit a GitHub tarball timeout so the
                // next retry can use --omit=optional as a fallback strategy.
                if is_github_tarball_error(&e) {
                    github_tarball_failed = true;
                }
                last_error = e.clone();
                // Clean up before retry — targeted fixes based on error type,
                // modeled after the official install.sh retry strategy.
                if attempt < max_retries {
                    emit_log(&window, ch, "Cleaning up before retry...");

                    // Resolve npm global root for cleanup operations
                    #[cfg(windows)]
                    let npm_prefix = {
                        Command::new("cmd")
                            .args(["/C", "npm", "config", "get", "prefix"])
                            .env("PATH", &path_snapshot)
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::null())
                            .output()
                            .await
                            .ok()
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                            .unwrap_or_default()
                    };
                    #[cfg(not(windows))]
                    let npm_prefix = {
                        Command::new("npm")
                            .args(["config", "get", "prefix"])
                            .env("PATH", &path_snapshot)
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::null())
                            .output()
                            .await
                            .ok()
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                            .unwrap_or_default()
                    };

                    if !npm_prefix.is_empty() {
                        let npm_root = std::path::PathBuf::from(&npm_prefix).join("lib").join("node_modules");
                        // Fallback: on Windows npm root is <prefix>/node_modules
                        let npm_root = if npm_root.exists() {
                            npm_root
                        } else {
                            std::path::PathBuf::from(&npm_prefix).join("node_modules")
                        };

                        let lower_err = e.to_lowercase();

                        // ENOTEMPTY: "directory not empty, rename" — stale .openclaw-* temp dirs.
                        // Official install.sh cleans .openclaw-* and openclaw under npm root.
                        if lower_err.contains("enotempty") {
                            emit_log(&window, ch, "ENOTEMPTY detected — cleaning stale staging directories...");
                            cleanup_npm_openclaw_dirs(&npm_root).await;
                        }

                        // EEXIST: "file already exists" — conflicting bin symlinks from a previous install.
                        // Official install.sh removes the conflicting file and retries.
                        if lower_err.contains("eexist") {
                            emit_log(&window, ch, "EEXIST detected — removing conflicting files...");
                            cleanup_npm_openclaw_dirs(&npm_root).await;
                            // Remove the openclaw bin symlink/script that conflicts.
                            // On Unix: <prefix>/bin/openclaw (symlink)
                            // On Windows: <prefix>/openclaw.cmd (script), no bin/ subdir
                            let bin_names: &[&str] = if cfg!(windows) {
                                &["openclaw", "openclaw.cmd", "openclaw.ps1"]
                            } else {
                                &["openclaw"]
                            };
                            let bin_dir = if cfg!(windows) {
                                std::path::PathBuf::from(&npm_prefix)
                            } else {
                                std::path::PathBuf::from(&npm_prefix).join("bin")
                            };
                            for name in bin_names {
                                let bin_path = bin_dir.join(name);
                                if bin_path.exists() {
                                    emit_log(&window, ch, &format!("Removing conflicting {}", bin_path.display()));
                                    let _ = tokio::fs::remove_file(&bin_path).await;
                                }
                            }
                        }

                        // General cleanup: remove leftover openclaw package dir
                        let leftover = npm_root.join("openclaw");
                        if leftover.exists() {
                            emit_log(&window, ch, &format!("Removing leftover {}", leftover.display()));
                            if cfg!(windows) {
                                let leftover_str = leftover.to_string_lossy().to_string();
                                for rm_attempt in 1u8..=3 {
                                    let ok = Command::new("cmd")
                                        .args(["/C", "rmdir", "/s", "/q", &leftover_str])
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .status()
                                        .await
                                        .map(|s| s.success())
                                        .unwrap_or(false);
                                    if ok || !leftover.exists() { break; }
                                    if rm_attempt < 3 {
                                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                    }
                                }
                            } else {
                                let _ = tokio::fs::remove_dir_all(&leftover).await;
                            }
                        }
                    }

                    // Clean npm cache on ENOTEMPTY/EEXIST or as general fallback
                    #[cfg(windows)]
                    let _ = Command::new("cmd")
                        .args(["/C", "npm", "cache", "clean", "--force"])
                        .env("PATH", &path_snapshot)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .output().await;
                    #[cfg(not(windows))]
                    let _ = Command::new("npm")
                        .args(["cache", "clean", "--force"])
                        .env("PATH", &path_snapshot)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .output().await;
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

    // Verify openclaw binary is actually callable before declaring success.
    // npm exit 0 doesn't guarantee the binary is in PATH.
    refresh_system_path();
    let oc_verify = cmd("openclaw")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;
    if !oc_verify.map(|o| o.status.success()).unwrap_or(false) {
        emit_log(&window, ch, "Warning: openclaw binary not immediately found in PATH.");
        if cfg!(windows) {
            emit_log(&window, ch,
                "On Windows the new PATH entry takes effect after the app restarts. \
                 Please close and reopen this installer, then proceed to the next step.");
        } else {
            emit_log(&window, ch, "This may resolve after restarting the application.");
        }
    }

    // On Windows, persist the npm global bin directory to the user PATH registry so
    // openclaw is discoverable after the app restarts.
    #[cfg(windows)]
    {
        super::path_env::refresh_system_path();
        // Ask npm where globals live and persist that directory.
        if let Ok(prefix_out) = Command::new("cmd")
            .args(["/C", "npm", "config", "get", "prefix"])
            .env("PATH", super::path_env::expanded_path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .await
        {
            let prefix = String::from_utf8_lossy(&prefix_out.stdout).trim().to_string();
            if !prefix.is_empty() && !prefix.starts_with("npm") {
                append_dirs_to_user_path_registry(&[&prefix]);
            }
        }
    }

    // Step 3: Run openclaw doctor for migrations and health check.
    // The official install.sh always does this after installation; it handles
    // config migration, gateway refresh, and catches common setup issues.
    emit_progress(&window, ch, 85, "Running openclaw doctor...");
    emit_log(&window, ch, "Running openclaw doctor --non-interactive for config migration and health check...");
    let doctor_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        cmd("openclaw")
            .args(["doctor", "--non-interactive"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output(),
    )
    .await;
    match doctor_result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.trim().is_empty() {
                emit_log(&window, ch, &format!("doctor: {}", stdout.trim()));
            }
            if !stderr.trim().is_empty() {
                emit_log(&window, ch, &format!("doctor stderr: {}", stderr.trim()));
            }
            if output.status.success() {
                emit_log(&window, ch, "openclaw doctor completed successfully.");
            } else {
                emit_log(&window, ch, "openclaw doctor exited with warnings (non-fatal).");
            }
        }
        Ok(Err(e)) => {
            emit_log(&window, ch, &format!("openclaw doctor failed to run: {} (non-fatal)", e));
        }
        Err(_) => {
            emit_log(&window, ch, "openclaw doctor timed out after 30s (non-fatal).");
        }
    }

    // Step 4: Retrieve installed version
    emit_progress(&window, ch, 95, "Verifying installation...");
    let version = get_openclaw_version().await;
    emit_progress(&window, ch, 100, "OpenClaw installed successfully!");
    Ok(InstallResult { version })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_with_trailing_slash() {
        let url = build_node_download_url("https://npmmirror.com/mirrors/node/");
        assert!(url.starts_with("https://npmmirror.com/mirrors/node/v22.22.0/"));
        assert!(url.contains("node-v22.22.0"));
    }

    #[test]
    fn build_url_without_trailing_slash() {
        let url = build_node_download_url("https://npmmirror.com/mirrors/node");
        assert!(url.starts_with("https://npmmirror.com/mirrors/node/v22.22.0/"));
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
