//! Copilot: GitHub OAuth token from config.json (`providers[].apiKey` for `copilot`, written by the
//! device-flow sign-in in Settings), the Copilot IDE plugins (`%LOCALAPPDATA%\github-copilot`), or the
//! GitHub CLI (`gh auth token`), then `GET https://api.github.com/copilot_internal/user`
//! (see CopilotUsageFetcher.swift / CopilotDeviceFlow.swift).

use super::*;

pub const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

pub fn normalize_host(raw: Option<String>) -> String {
    let Some(h) = raw else { return "github.com".into() };
    let h = h.trim().trim_start_matches("https://").trim_start_matches("http://");
    let h = h.split('/').next().unwrap_or("").trim();
    if h.is_empty() { "github.com".into() } else { h.to_string() }
}

fn api_host(host: &str) -> String {
    if host == "github.com" { "api.github.com".into() } else { format!("api.{host}") }
}

fn plugin_token() -> Option<String> {
    let dirs = [
        crate::config::local_appdata_dir().join("github-copilot"),
        crate::config::home_dir().join(".config").join("github-copilot"),
    ];
    for dir in dirs {
        for file in ["apps.json", "hosts.json"] {
            if let Some(v) = read_json_file(&dir.join(file)) {
                if let Some(obj) = v.as_object() {
                    for (k, entry) in obj {
                        if k.starts_with("github.com") {
                            if let Some(t) = entry.get("oauth_token").and_then(|t| t.as_str()) {
                                return Some(t.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn gh_token() -> Option<String> {
    let out = hidden_command("gh").args(["auth", "token"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let t = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!t.is_empty()).then_some(t)
}

fn quota_window(label: &str, q: Option<&serde_json::Value>, reset: Option<i64>) -> Option<UsageWindow> {
    let q = q?.as_object()?;
    if q.get("unlimited").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let ent = num(q.get("entitlement"));
    let rem = num(q.get("remaining"));
    if ent == Some(0.0) && rem == Some(0.0) {
        return None; // placeholder snapshot
    }
    let pct_remaining = num(q.get("percent_remaining")).or_else(|| match (ent, rem) {
        (Some(e), Some(r)) if e > 0.0 => Some(r / e * 100.0),
        _ => None,
    })?;
    Some(UsageWindow {
        label: label.into(),
        used_percent: (100.0 - pct_remaining).max(0.0),
        resets_at: reset,
        detail: match (ent, rem) {
            (Some(e), Some(r)) if e > 0.0 => Some(format!("{:.0} / {:.0} used", (e - r).max(0.0), e)),
            _ => None,
        },
    })
}

pub fn fetch(cfg: &SharedConfig) -> FetchResult {
    let host = normalize_host(cfg.provider_str("copilot", "enterpriseHost"));
    let token = cfg
        .provider_str("copilot", "apiKey")
        .or_else(|| std::env::var("COPILOT_GITHUB_TOKEN").ok().filter(|s| !s.is_empty()))
        .or_else(|| if host == "github.com" { plugin_token() } else { None })
        .or_else(|| if host == "github.com" { gh_token() } else { None })
        .ok_or_else(|| {
            FetchError::NotConfigured("No GitHub token. Use Settings → Copilot → Sign in with GitHub.".into())
        })?;

    let json = json_response(
        agent()
            .get(&format!("https://{}/copilot_internal/user", api_host(&host)))
            .set("Authorization", &format!("token {token}"))
            .set("Accept", "application/json")
            .set("Editor-Version", "vscode/1.96.2")
            .set("Editor-Plugin-Version", "copilot-chat/0.26.7")
            .set("User-Agent", "GitHubCopilotChat/0.26.7")
            .set("X-Github-Api-Version", "2025-04-01")
            .call(),
        "GitHub token has no Copilot access. Sign in again in Settings → Copilot.",
    )?;

    let reset = json.get("quota_reset_date").and_then(|v| v.as_str()).and_then(parse_iso);
    let snaps = json.get("quota_snapshots");
    let mut snap = ProviderSnapshot::new("copilot", "Copilot");
    snap.plan = json.get("copilot_plan").and_then(|v| v.as_str()).map(capitalize);
    snap.account = json.get("login").and_then(|v| v.as_str()).map(String::from);
    if let Some(w) = quota_window("Premium requests", snaps.and_then(|s| s.get("premium_interactions")), reset) {
        snap.windows.push(w);
    }
    if let Some(w) = quota_window("Chat", snaps.and_then(|s| s.get("chat")), reset) {
        snap.windows.push(w);
    }
    if snap.windows.is_empty() {
        snap.windows.push(UsageWindow {
            label: "Premium requests".into(),
            used_percent: 0.0,
            resets_at: reset,
            detail: Some("Unlimited / not metered".into()),
        });
    }
    Ok(snap)
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

pub fn device_start(host: &str) -> Result<DeviceCode, String> {
    let resp = agent()
        .post(&format!("https://{host}/login/device/code"))
        .set("Accept", "application/json")
        .send_form(&[("client_id", CLIENT_ID), ("scope", "read:user")])
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    Ok(DeviceCode {
        device_code: v.get("device_code").and_then(|x| x.as_str()).ok_or("missing device_code")?.into(),
        user_code: v.get("user_code").and_then(|x| x.as_str()).ok_or("missing user_code")?.into(),
        verification_uri: v
            .get("verification_uri")
            .and_then(|x| x.as_str())
            .unwrap_or("https://github.com/login/device")
            .into(),
        interval: v.get("interval").and_then(|x| x.as_u64()).unwrap_or(5),
        expires_in: v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(900),
    })
}

/// Blocks (on a worker thread) until the user approves the code, it expires, or is denied.
pub fn device_poll(host: &str, code: &DeviceCode) -> Result<String, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
    let mut interval = code.interval.max(1);
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_secs(interval));
        let resp = agent()
            .post(&format!("https://{host}/login/oauth/access_token"))
            .set("Accept", "application/json")
            .send_form(&[
                ("client_id", CLIENT_ID),
                ("device_code", code.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .map_err(|e| e.to_string())?;
        let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        if let Some(t) = v.get("access_token").and_then(|x| x.as_str()) {
            return Ok(t.to_string());
        }
        match v.get("error").and_then(|x| x.as_str()) {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += 5,
            Some("access_denied") => return Err("Sign-in was cancelled.".into()),
            Some("expired_token") => return Err("The code expired. Try again.".into()),
            Some(other) => return Err(other.to_string()),
            None => return Err("Unexpected response from GitHub.".into()),
        }
    }
    Err("The code expired. Try again.".into())
}
