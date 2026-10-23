//! Provider fetchers. Each one mirrors the request/response shapes used by
//! CodexBarCore (Sources/CodexBarCore/Providers/<Name>) for the same source.

pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;

use crate::config::SharedConfig;
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub label: String,
    /// 0..=100 (may exceed 100 when a provider reports overage).
    pub used_percent: f64,
    /// Unix seconds.
    pub resets_at: Option<i64>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub id: &'static str,
    pub name: &'static str,
    pub plan: Option<String>,
    pub account: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub error: Option<String>,
    /// True when no local credentials exist (shown as "not set up", not as an error).
    pub not_configured: bool,
    pub updated_at: i64,
}

impl ProviderSnapshot {
    pub fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            plan: None,
            account: None,
            windows: vec![],
            error: None,
            not_configured: false,
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo { id: "codex", name: "Codex" },
    ProviderInfo { id: "claude", name: "Claude" },
    ProviderInfo { id: "cursor", name: "Cursor" },
    ProviderInfo { id: "gemini", name: "Gemini" },
    ProviderInfo { id: "copilot", name: "Copilot" },
];

pub enum FetchError {
    NotConfigured(String),
    Failed(String),
}

pub type FetchResult = Result<ProviderSnapshot, FetchError>;

pub fn fetch(id: &str, cfg: &SharedConfig) -> ProviderSnapshot {
    let info = PROVIDERS.iter().find(|p| p.id == id).expect("known provider");
    let result = match id {
        "codex" => codex::fetch(cfg),
        "claude" => claude::fetch(cfg),
        "cursor" => cursor::fetch(cfg),
        "gemini" => gemini::fetch(cfg),
        "copilot" => copilot::fetch(cfg),
        _ => Err(FetchError::Failed("unknown provider".into())),
    };
    match result {
        Ok(s) => s,
        Err(FetchError::NotConfigured(msg)) => {
            let mut s = ProviderSnapshot::new(info.id, info.name);
            s.not_configured = true;
            s.error = Some(msg);
            s
        }
        Err(FetchError::Failed(msg)) => {
            let mut s = ProviderSnapshot::new(info.id, info.name);
            s.error = Some(msg);
            s
        }
    }
}

pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(25))
            .user_agent("CodexBar-Windows/0.1")
            .build()
    })
}

/// Turns a ureq result into JSON, mapping HTTP status codes into readable errors.
pub fn json_response(
    result: Result<ureq::Response, ureq::Error>,
    auth_hint: &str,
) -> Result<serde_json::Value, FetchError> {
    match result {
        Ok(resp) => resp
            .into_json::<serde_json::Value>()
            .map_err(|e| FetchError::Failed(format!("Invalid response: {e}"))),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            let snippet: String = body.chars().take(160).collect();
            Err(match code {
                401 | 403 => FetchError::Failed(format!("HTTP {code}: {auth_hint}")),
                429 => FetchError::Failed("Rate limited by provider (HTTP 429); will retry later.".into()),
                _ => FetchError::Failed(format!("HTTP {code}: {snippet}")),
            })
        }
        Err(e) => Err(FetchError::Failed(format!("Network error: {e}"))),
    }
}

pub fn read_json_file(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// Decodes the payload of a JWT without verifying it (identity hints only).
pub fn jwt_claims(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn parse_iso(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp());
    }
    None
}

pub fn num(v: Option<&serde_json::Value>) -> Option<f64> {
    match v? {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().replace(['_', '-'], " "),
        None => String::new(),
    }
}

#[cfg(windows)]
pub fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut c = std::process::Command::new(program);
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    c
}

#[cfg(not(windows))]
pub fn hidden_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}
