//! Claude: Claude Code OAuth credentials from `%USERPROFILE%\.claude\.credentials.json`
//! (or `$CLAUDE_CONFIG_DIR`), then `GET https://api.anthropic.com/api/oauth/usage`
//! (see ClaudeOAuthUsageFetcher.swift). Tokens are read-only; Claude Code refreshes them.

use super::*;
use std::path::PathBuf;

fn claude_dir() -> PathBuf {
    std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".claude"))
}

const WINDOWS: &[(&str, &str)] = &[
    ("five_hour", "Session (5h)"),
    ("seven_day", "Weekly"),
    ("seven_day_opus", "Weekly Opus"),
    ("seven_day_sonnet", "Weekly Sonnet"),
];

pub fn fetch(_cfg: &SharedConfig) -> FetchResult {
    let path = claude_dir().join(".credentials.json");
    let creds = read_json_file(&path).ok_or_else(|| {
        FetchError::NotConfigured(format!("No Claude Code login found ({}). Run `claude` and sign in.", path.display()))
    })?;
    let oauth = creds
        .get("claudeAiOauth")
        .ok_or_else(|| FetchError::NotConfigured("Claude credentials have no claudeAiOauth entry.".into()))?;
    let token = oauth
        .get("accessToken")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| FetchError::NotConfigured("Claude credentials have no access token.".into()))?;
    if let Some(exp) = num(oauth.get("expiresAt")) {
        if (exp / 1000.0) < chrono::Utc::now().timestamp() as f64 {
            return Err(FetchError::Failed(
                "Claude Code token expired. Open `claude` once so it refreshes the login.".into(),
            ));
        }
    }

    let resp = agent()
        .get("https://api.anthropic.com/api/oauth/usage")
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .set("anthropic-beta", "oauth-2025-04-20")
        .set("User-Agent", "claude-code/2.1.0")
        .call();
    let json = json_response(resp, "Claude login expired or lacks usage scope. Run `claude` and sign in again.")?;

    let mut snap = ProviderSnapshot::new("claude", "Claude");
    snap.plan = oauth.get("subscriptionType").and_then(|v| v.as_str()).map(capitalize);
    for (key, label) in WINDOWS {
        if let Some(w) = json.get(*key).filter(|w| w.is_object()) {
            let Some(util) = num(w.get("utilization")) else { continue };
            snap.windows.push(UsageWindow {
                label: (*label).into(),
                used_percent: util,
                resets_at: w.get("resets_at").and_then(|v| v.as_str()).and_then(parse_iso),
                detail: None,
            });
        }
    }
    if let Some(extra) = json.get("extra_usage").filter(|e| e.get("is_enabled").and_then(|v| v.as_bool()) == Some(true)) {
        if let (Some(used), Some(limit)) = (num(extra.get("used_credits")), num(extra.get("monthly_limit"))) {
            if limit > 0.0 {
                snap.windows.push(UsageWindow {
                    label: "Extra usage".into(),
                    used_percent: used / limit * 100.0,
                    resets_at: None,
                    detail: Some(format!("${:.2} / ${:.2}", used / 100.0, limit / 100.0)),
                });
            }
        }
    }
    if snap.windows.is_empty() {
        snap.error = Some("Claude returned no usage windows for this account.".into());
    }
    Ok(snap)
}
