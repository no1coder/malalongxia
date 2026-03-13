use std::path::PathBuf;

#[cfg(windows)]
mod npm_prefix_cache {
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

    fn mutex() -> &'static Mutex<Option<PathBuf>> {
        CACHE.get_or_init(|| Mutex::new(None))
    }

    pub fn get_or_query() -> Option<PathBuf> {
        let m = mutex();
        let cached = m.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(p) = cached {
            return Some(p);
        }
        // Use spawn + try_wait loop instead of .output() so we can enforce
        // a timeout. A broken/hung npm must never block the entire app.
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "npm", "config", "get", "prefix"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        // Poll for up to 3 seconds (30 × 100ms)
        let status = {
            let mut exit_status = None;
            for _ in 0..30 {
                match child.try_wait() {
                    Ok(Some(s)) => { exit_status = Some(s); break; }
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                    Err(_) => break,
                }
            }
            if exit_status.is_none() {
                let _ = child.kill();
                let _ = child.wait(); // reap
            }
            exit_status
        };

        let result = status
            .filter(|s| s.success())
            .and_then(|_| {
                use std::io::Read;
                let mut output = String::new();
                child.stdout.take()?.read_to_string(&mut output).ok()?;
                let prefix = output.trim().to_string();
                if !prefix.is_empty() && !prefix.starts_with("npm") {
                    Some(PathBuf::from(prefix))
                } else {
                    None
                }
            });

        if let Some(ref p) = result {
            *m.lock().unwrap_or_else(|e| e.into_inner()) = Some(p.clone());
        }
        result
    }

    pub fn invalidate() {
        *mutex().lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Build an expanded PATH string that includes common Node.js install locations.
///
/// Packaged desktop apps (especially macOS .app bundles) don't inherit the user's
/// shell profile (~/.zshrc, ~/.bashrc), so tools installed via nvm, fnm, Homebrew,
/// Volta, etc. won't be found. This function adds those directories explicitly.
///
/// Cross-platform: uses `;` on Windows, `:` on Unix.
pub fn expanded_path() -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    let home = dirs::home_dir().unwrap_or_default();

    let mut extra: Vec<PathBuf> = Vec::new();

    #[cfg(unix)]
    {
        // Homebrew (macOS ARM + Intel)
        extra.push(PathBuf::from("/opt/homebrew/bin"));
        extra.push(PathBuf::from("/opt/homebrew/sbin"));
        extra.push(PathBuf::from("/usr/local/bin"));

        // nvm: ~/.nvm/versions/node/*/bin (pick latest)
        push_glob_latest(&mut extra, &home.join(".nvm/versions/node/*/bin"));

        // fnm: ~/.fnm/node-versions/*/installation/bin
        push_glob_latest(&mut extra, &home.join(".fnm/node-versions/*/installation/bin"));

        // Volta
        extra.push(home.join(".volta/bin"));

        // n (tj/n)
        extra.push(home.join("n/bin"));

        // User local bin
        extra.push(home.join(".local/bin"));

        // Direct install locations used by this app
        extra.push(home.join(".local/node/bin"));
        extra.push(home.join(".local/git/bin"));
    }

    #[cfg(windows)]
    {
        // Default Node.js installer location
        extra.push(PathBuf::from(r"C:\Program Files\nodejs"));
        extra.push(PathBuf::from(r"C:\Program Files (x86)\nodejs"));

        // npm global bin — try the static default first, then query npm for the actual prefix
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            extra.push(PathBuf::from(&appdata).join("npm"));
        }

        // Dynamically resolve npm global prefix so custom npm prefix locations are found.
        if let Some(p) = npm_prefix_cache::get_or_query() {
            extra.push(p);
        }

        // nvm-windows
        let nvm_home = std::env::var("NVM_HOME").unwrap_or_default();
        if !nvm_home.is_empty() {
            extra.push(PathBuf::from(&nvm_home));
        }
        let nvm_symlink = std::env::var("NVM_SYMLINK").unwrap_or_default();
        if !nvm_symlink.is_empty() {
            extra.push(PathBuf::from(&nvm_symlink));
        }

        // fnm on Windows: check FNM_DIR env var first, then fall back to default locations.
        // fnm_multishells is a temporary shell-proxy dir, not where node binaries live.
        let local_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        if !local_data.is_empty() {
            // fnm stores Node versions under %LOCALAPPDATA%\fnm\node-versions\<ver>\installation
            let fnm_base = PathBuf::from(&local_data).join("fnm").join("node-versions");
            push_latest_fnm_node(&mut extra, &fnm_base);
        }
        // Also honour FNM_DIR if the user configured a custom location
        let fnm_dir = std::env::var("FNM_DIR").unwrap_or_default();
        if !fnm_dir.is_empty() {
            let fnm_base = PathBuf::from(&fnm_dir).join("node-versions");
            push_latest_fnm_node(&mut extra, &fnm_base);
        }

        // Volta on Windows
        extra.push(home.join(".volta").join("bin"));

        // Scoop
        extra.push(home.join("scoop").join("shims"));

        // Git for Windows
        extra.push(PathBuf::from(r"C:\Program Files\Git\cmd"));
        extra.push(PathBuf::from(r"C:\Program Files (x86)\Git\cmd"));

        // MinGit (portable git installed by this app)
        if !local_data.is_empty() {
            extra.push(PathBuf::from(&local_data).join("Programs").join("MinGit").join("cmd"));
        }

        // Portable Node.js (installed by this app)
        if !local_data.is_empty() {
            extra.push(PathBuf::from(&local_data).join("Programs").join("nodejs"));
        }
    }

    // Only keep paths that actually exist
    let mut parts: Vec<String> = extra
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    parts.push(current_path);

    let sep = if cfg!(windows) { ";" } else { ":" };
    parts.join(sep)
}

/// Invalidate the cached npm prefix so the next call to expanded_path() re-queries npm.
/// Call this after installing Node.js so the new npm global bin directory is discovered.
#[cfg(windows)]
pub fn invalidate_npm_prefix_cache() {
    npm_prefix_cache::invalidate();
}

#[cfg(not(windows))]
pub fn invalidate_npm_prefix_cache() {}

/// Refresh the expanded PATH by re-reading system environment variables.
/// On Windows, after an MSI installer modifies the system PATH, the current
/// process still has the old PATH. This function reads the latest Machine and
/// User PATH values from the Windows registry and merges them so that freshly
/// installed programs (e.g. node/npm) become discoverable.
#[cfg(windows)]
pub fn refresh_system_path() {
    use std::collections::HashSet;
    use std::process::Command as StdCommand;

    // Read Machine PATH from registry
    let machine_path = StdCommand::new("reg")
        .args(["query", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment", "/v", "Path"])
        .output()
        .ok()
        .and_then(|o| parse_reg_value(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();

    // Read User PATH from registry
    let user_path = StdCommand::new("reg")
        .args(["query", r"HKCU\Environment", "/v", "Path"])
        .output()
        .ok()
        .and_then(|o| parse_reg_value(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();

    if machine_path.is_empty() && user_path.is_empty() {
        return;
    }

    // Merge registry paths with current process PATH, deduplicating.
    // Registry entries come first (freshest system state), then any
    // process-only entries (e.g. MinGit added at runtime) are appended.
    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut seen = HashSet::new();
    let mut merged: Vec<&str> = Vec::new();

    for source in [machine_path.as_str(), user_path.as_str(), current_path.as_str()] {
        for entry in source.split(';') {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                let lower = trimmed.to_lowercase();
                if seen.insert(lower) {
                    merged.push(trimmed);
                }
            }
        }
    }

    std::env::set_var("PATH", merged.join(";"));
}

/// Parse a registry value from `reg query` output.
/// Output format: `    Path    REG_EXPAND_SZ    C:\Windows\system32;...`
/// Delimiters can be tabs or multiple spaces; we split on REG_SZ/REG_EXPAND_SZ.
#[cfg(windows)]
fn parse_reg_value(output: &str) -> Option<String> {
    output.lines()
        .find(|l| l.contains("REG_"))
        .and_then(|line| {
            // Find the position after "REG_SZ" or "REG_EXPAND_SZ"
            if let Some(pos) = line.find("REG_EXPAND_SZ") {
                Some(line[pos + "REG_EXPAND_SZ".len()..].trim().to_string())
            } else if let Some(pos) = line.find("REG_SZ") {
                Some(line[pos + "REG_SZ".len()..].trim().to_string())
            } else {
                None
            }
        })
        .filter(|v| !v.is_empty())
}

#[cfg(not(windows))]
pub fn refresh_system_path() {
    // On Unix, PATH changes are applied via shell profile; no-op here.
    // expanded_path() already scans common locations dynamically.
}

/// Resolve a glob pattern and push the latest (highest version) match into `out`.
/// Uses semantic version comparison so v22.x sorts after v9.x.
#[cfg(unix)]
fn push_glob_latest(out: &mut Vec<PathBuf>, pattern: &std::path::Path) {
    let pattern_str = pattern.to_string_lossy();
    if let Ok(entries) = glob::glob(&pattern_str) {
        let mut matches: Vec<PathBuf> = entries.filter_map(|e| e.ok()).collect();
        matches.sort_by(|a, b| {
            let ver_a = extract_version_tuple(a);
            let ver_b = extract_version_tuple(b);
            ver_a.cmp(&ver_b)
        });
        if let Some(last) = matches.pop() {
            out.push(last);
        }
    }
}

/// Extract a (major, minor, patch) version tuple from a path like
/// `.nvm/versions/node/v22.14.0/bin` for semantic comparison.
/// Falls back to (0, 0, 0) if no version pattern is found.
pub fn version_tuple_from_path(path: &std::path::Path) -> (u64, u64, u64) {
    extract_version_tuple(path)
}

fn extract_version_tuple(path: &std::path::Path) -> (u64, u64, u64) {
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        // Match version directory names like "v22.14.0" or "22.14.0"
        let trimmed = s.strip_prefix('v').unwrap_or(&s);
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                let patch = parts.get(2).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
                return (major, minor, patch);
            }
        }
    }
    (0, 0, 0)
}

/// Find the highest-versioned fnm Node installation under `base/node-versions/<ver>/installation`
/// and push it onto `out`. Works on Windows where glob crate may not be available.
#[cfg(windows)]
fn push_latest_fnm_node(out: &mut Vec<PathBuf>, base: &PathBuf) {
    if !base.exists() {
        return;
    }
    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut best: Option<((u64, u64, u64), PathBuf)> = None;
    for entry in entries.flatten() {
        let installation = entry.path().join("installation");
        if installation.exists() {
            let ver = extract_version_tuple(&entry.path());
            if ver > (0, 0, 0) {
                match best {
                    Some((ref b, _)) if ver > *b => best = Some((ver, installation)),
                    None => best = Some((ver, installation)),
                    _ => {}
                }
            }
        }
    }
    if let Some((_, path)) = best {
        out.push(path);
    }
}
