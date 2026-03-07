use std::path::PathBuf;

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

        // npm global bin
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            extra.push(PathBuf::from(&appdata).join("npm"));
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

        // fnm on Windows
        let local_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        if !local_data.is_empty() {
            extra.push(PathBuf::from(&local_data).join("fnm_multishells"));
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
