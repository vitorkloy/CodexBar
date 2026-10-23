//! Codex: OAuth tokens from `%USERPROFILE%\.codex\auth.json` (or `$CODEX_HOME`), then
//! `GET https://chatgpt.com/backend-api/wham/usage` (see CodexOAuthUsageFetcher.swift).
//! Like upstream, tokens are never refreshed or written back to `auth.json`.

use super::*;
use std::path::PathBuf;

fn codex_home() -> PathBuf {
    std::env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".codex"))
}

fn base_url() -> String {
    // Honour `chatgpt_base_url` in config.toml (uncommented lines only).
    let toml = std::fs::read_to_string(codex_home().join("config.toml")).unwrap_or_default();
    for line in toml.lines() {
        let l = line.trim();
        if l.starts_with('#') {
            continue;
        }
        if let Some(rest) = l.strip_prefix("chatgpt_base_url") {
            if let Some(v) = rest.trim().strip_prefix('=') {
                let v = v.trim().trim_matches('"').trim_end_matches('/');
                if v.starts_with("https://") {
                    return v.to_string();
                }
            }
        }
    }
    "https://chatgpt.com/backend-api".into()
}

fn window_label(seconds: Option<f64>, fallback: &str) -> String {
    match seconds.map(|s| (s / 60.0).round() as i64) {
        Some(300) => "Session (5h)".into(),
        Some(10080) => "Weekly".into(),
        Some(m) if m > 0 && m % 1440 == 0 => format!("{} days", m / 1440),
        Some(m) if m > 0 && m % 60 == 0 => format!("{}h window", m / 60),
        _ => fallback.into(),
    }
}

pub fn fetch(_cfg: &SharedConfig) -> FetchResult {
    let path = codex_home().join("auth.json");
    let auth = read_json_file(&path).ok_or_else(|| {
        FetchError::NotConfigured(format!("No Codex login found ({}). Run `codex login`.", path.display()))
    })?;
    let tokens = auth.get("tokens").and_then(|t| t.as_object());
    let access = tokens
        .and_then(|t| t.get("access_token").or_else(|| t.get("accessToken")))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let Some(access) = access else {
        if auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()).is_some() {
            return Err(FetchError::NotConfigured(
                "Codex is signed in with an API key; usage limits need a ChatGPT login (`codex login`).".into(),
            ));
        }
        return Err(FetchError::NotConfigured("Codex auth.json has no OAuth tokens. Run `codex login`.".into()));
    };
    let id_claims = tokens
        .and_then(|t| t.get("id_token").or_else(|| t.get("idToken")))
        .and_then(|v| v.as_str())
        .and_then(jwt_claims);
    let account_id = tokens
        .and_then(|t| t.get("account_id").or_else(|| t.get("accountId")))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            let c = id_claims.as_ref()?;
            c.get("chatgpt_account_id")
                .or_else(|| c.get("https://api.openai.com/auth")?.get("chatgpt_account_id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });

    let mut req = agent()
        .get(&format!("{}/wham/usage", base_url()))
        .set("Authorization", &format!("Bearer {access}"))
        .set("User-Agent", "CodexBar")
        .set("Accept", "application/json");
    if let Some(id) = &account_id {
        req = req.set("ChatGPT-Account-Id", id);
    }
    let json = json_response(req.call(), "Codex login expired. Run `codex` once to refresh it.")?;

    let mut snap = ProviderSnapshot::new("codex", "Codex");
    snap.plan = json.get("plan_type").and_then(|v| v.as_str()).map(capitalize);
    snap.account = id_claims.as_ref().and_then(|c| c.get("email")).and_then(|v| v.as_str()).map(String::from);
    if let Some(rl) = json.get("rate_limit") {
        for (key, fallback) in [("primary_window", "Session"), ("secondary_window", "Weekly")] {
            if let Some(w) = rl.get(key).filter(|w| w.is_object()) {
                let used = num(w.get("used_percent")).unwrap_or(0.0);
                let reset = num(w.get("reset_at")).map(|v| v as i64).or_else(|| {
                    num(w.get("reset_after_seconds")).map(|s| chrono::Utc::now().timestamp() + s as i64)
                });
                snap.windows.push(UsageWindow {
                    label: window_label(num(w.get("limit_window_seconds")), fallback),
                    used_percent: used,
                    resets_at: reset,
                    detail: None,
                });
            }
        }
    }
    if let Some(credits) = json.get("credits") {
        if credits.get("has_credits").and_then(|v| v.as_bool()) == Some(true) {
            if let Some(balance) = num(credits.get("balance")) {
                snap.windows.push(UsageWindow {
                    label: "Credits".into(),
                    used_percent: 0.0,
                    resets_at: None,
                    detail: Some(format!("{balance:.0} credits left")),
                });
            }
        }
    }
    if snap.windows.is_empty() {
        snap.error = Some("Codex returned no rate-limit windows for this account.".into());
    }
    Ok(snap)
}
