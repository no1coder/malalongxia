use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct MirrorResult {
    pub name: String,
    pub url: String,
    pub latency_ms: Option<u64>,
    pub reachable: bool,
}

// Remote mirror configuration fetched from yuan.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorEntry {
    pub name: String,
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMirrorConfig {
    pub version: u32,
    #[serde(default)]
    pub updated_at: String,
    pub node_mirrors: Vec<MirrorEntry>,
    pub npm_mirrors: Vec<MirrorEntry>,
    #[serde(default)]
    pub nvm_install_script: Option<String>,
    #[serde(default)]
    pub node_version: Option<String>,
}

// Built-in mirror configuration (no remote fetch needed)
const NODE_MIRRORS: &[(&str, &str)] = &[
    ("mirror.aliyun", "https://npmmirror.com/mirrors/node/"),
    ("mirror.tencent", "https://mirrors.cloud.tencent.com/nodejs-release/"),
    ("mirror.tsinghua", "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/"),
    ("mirror.huawei", "https://repo.huaweicloud.com/nodejs/"),
];

const NPM_MIRRORS: &[(&str, &str)] = &[
    ("mirror.npmmirror", "https://registry.npmmirror.com"),
    ("mirror.tencent", "https://mirrors.cloud.tencent.com/npm/"),
    ("mirror.huawei", "https://repo.huaweicloud.com/repository/npm/"),
    ("mirror.official", "https://registry.npmjs.org"),
];

fn build_config() -> RemoteMirrorConfig {
    RemoteMirrorConfig {
        version: 1,
        updated_at: String::new(),
        node_mirrors: NODE_MIRRORS
            .iter()
            .map(|(name, url)| MirrorEntry {
                name: name.to_string(),
                url: url.to_string(),
                enabled: true,
            })
            .collect(),
        npm_mirrors: NPM_MIRRORS
            .iter()
            .map(|(name, url)| MirrorEntry {
                name: name.to_string(),
                url: url.to_string(),
                enabled: true,
            })
            .collect(),
        nvm_install_script: Some("https://gitee.com/mirrors/nvm/raw/master/install.sh".to_string()),
        node_version: None,
    }
}

/// Return built-in mirror configuration (no remote fetch).
#[tauri::command]
pub async fn fetch_mirror_config() -> Result<RemoteMirrorConfig, String> {
    Ok(build_config())
}

// Legacy hardcoded mirrors for backward compatibility with test_mirrors command
const MIRRORS: &[(&str, &str)] = &[
    ("aliyun", "https://npmmirror.com"),
    ("tencent", "https://mirrors.cloud.tencent.com"),
    ("tsinghua", "https://mirrors.tuna.tsinghua.edu.cn"),
    ("huawei", "https://mirrors.huaweicloud.com"),
];

// Test a single mirror and return latency info using HEAD request.
async fn test_single_mirror(
    client: &reqwest::Client,
    name: &str,
    url: &str,
) -> MirrorResult {
    let start = Instant::now();
    let result = client.head(url).send().await;
    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => MirrorResult {
            name: name.to_string(),
            url: url.to_string(),
            latency_ms: Some(elapsed),
            reachable: true,
        },
        // Some mirrors return 405 for HEAD but are still reachable
        Ok(resp) if resp.status().as_u16() == 405 => MirrorResult {
            name: name.to_string(),
            url: url.to_string(),
            latency_ms: Some(elapsed),
            reachable: true,
        },
        _ => MirrorResult {
            name: name.to_string(),
            url: url.to_string(),
            latency_ms: None,
            reachable: false,
        },
    }
}

#[tauri::command]
pub async fn test_mirrors() -> Result<Vec<MirrorResult>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut handles = Vec::new();
    for &(name, url) in MIRRORS {
        let c = client.clone();
        let n = name.to_string();
        let u = url.to_string();
        handles.push(tokio::spawn(async move {
            test_single_mirror(&c, &n, &u).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(r) => results.push(r),
            Err(e) => return Err(format!("Mirror test task failed: {}", e)),
        }
    }

    // Sort by latency (reachable first, then by ms)
    results.sort_by(|a, b| {
        match (a.latency_ms, b.latency_ms) {
            (Some(la), Some(lb)) => la.cmp(&lb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    Ok(results)
}

// Test latency for a single URL provided by the frontend (https only).
// Uses HEAD request with short timeout to measure connectivity, not page load.
#[tauri::command]
pub async fn test_mirror_latency(url: String) -> Result<u64, String> {
    if !url.starts_with("https://") {
        return Err("Only https:// URLs are allowed for mirror testing".to_string());
    }
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| e.to_string())?;
    client.head(&url).send().await.map_err(|e| e.to_string())?;
    Ok(start.elapsed().as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_list_has_four_entries() {
        assert_eq!(MIRRORS.len(), 4);
    }

    #[test]
    fn all_mirror_urls_are_https() {
        for &(_, url) in MIRRORS {
            assert!(url.starts_with("https://"), "Mirror URL not https: {}", url);
        }
    }

    #[tokio::test]
    async fn test_mirror_latency_rejects_http() {
        let result = test_mirror_latency("http://example.com".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Only https://"));
    }

    #[tokio::test]
    async fn test_mirror_latency_rejects_non_url() {
        let result = test_mirror_latency("ftp://example.com".to_string()).await;
        assert!(result.is_err());
    }

    #[test]
    fn mirror_result_sort_order() {
        let mut results = vec![
            MirrorResult {
                name: "c".to_string(),
                url: "https://c.com".to_string(),
                latency_ms: None,
                reachable: false,
            },
            MirrorResult {
                name: "a".to_string(),
                url: "https://a.com".to_string(),
                latency_ms: Some(50),
                reachable: true,
            },
            MirrorResult {
                name: "b".to_string(),
                url: "https://b.com".to_string(),
                latency_ms: Some(100),
                reachable: true,
            },
        ];

        results.sort_by(|a, b| match (a.latency_ms, b.latency_ms) {
            (Some(la), Some(lb)) => la.cmp(&lb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        assert_eq!(results[0].name, "a");
        assert_eq!(results[1].name, "b");
        assert_eq!(results[2].name, "c");
    }
}
