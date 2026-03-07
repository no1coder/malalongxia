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

/// Resolve a glob pattern and push the last (latest) match into `out`.
#[cfg(unix)]
fn push_glob_latest(out: &mut Vec<PathBuf>, pattern: &std::path::Path) {
    let pattern_str = pattern.to_string_lossy();
    if let Ok(entries) = glob::glob(&pattern_str) {
        let mut matches: Vec<PathBuf> = entries.filter_map(|e| e.ok()).collect();
        matches.sort();
        if let Some(last) = matches.pop() {
            out.push(last);
        }
    }
}
