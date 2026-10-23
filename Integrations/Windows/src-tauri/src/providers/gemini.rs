//! Gemini: Gemini CLI OAuth credentials (`%USERPROFILE%\.gemini\oauth_creds.json`), then
//! `loadCodeAssist` + `retrieveUserQuota` on cloudcode-pa.googleapis.com (see GeminiStatusProbe.swift).
//! Expired access tokens are refreshed in memory with the OAuth client shipped in the installed
//! Gemini CLI (or GEMINI_OAUTH_CLIENT_ID / GEMINI_OAUTH_CLIENT_SECRET); nothing is written back.

use super::*;
use std::path::PathBuf;
use std::sync::Mutex;

static TOKEN_CACHE: Mutex<Option<(String, i64)>> = Mutex::new(None);
static CLIENT_CACHE: Mutex<Option<(String, String)>> = Mutex::new(None);

fn gemini_dir() -> PathBuf {
    crate::config::home_dir().join(".gemini")
}

fn npm_roots() -> Vec<PathBuf> {
    let mut roots = vec![crate::config::appdata_dir().join("npm").join("node_modules")];
    if let Ok(prefix) = std::env::var("NPM_CONFIG_PREFIX") {
        roots.push(PathBuf::from(prefix).join("node_modules"));
    }
    roots.push(crate::config::local_appdata_dir().join("pnpm").join("global").join("5").join("node_modules"));
    roots
}

fn parse_client(content: &str) -> Option<(String, String)> {
    let id = regex::Regex::new(r#"OAUTH_CLIENT_ID\s*=\s*['"]([\w\-\.]+)['"]"#).ok()?;
    let secret = regex::Regex::new(r#"OAUTH_CLIENT_SECRET\s*=\s*['"]([\w\-]+)['"]"#).ok()?;
    Some((id.captures(content)?[1].to_string(), secret.captures(content)?[1].to_string()))
}

fn find_client() -> Option<(String, String)> {
    if let (Ok(id), Ok(secret)) = (std::env::var("GEMINI_OAUTH_CLIENT_ID"), std::env::var("GEMINI_OAUTH_CLIENT_SECRET")) {
        if !id.trim().is_empty() && !secret.trim().is_empty() {
            return Some((id.trim().into(), secret.trim().into()));
        }
    }
    if let Some(c) = CLIENT_CACHE.lock().ok()?.clone() {
        return Some(c);
    }
    let mut files: Vec<PathBuf> = vec![];
    for root in npm_roots() {
        let cli = root.join("@google").join("gemini-cli");
        for core in [
            cli.join("node_modules").join("@google").join("gemini-cli-core"),
            root.join("@google").join("gemini-cli-core"),
        ] {
            files.push(core.join("dist").join("src").join("code_assist").join("oauth2.js"));
        }
        if let Ok(rd) = std::fs::read_dir(cli.join("bundle")) {
            files.extend(rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "js")));
        }
    }
    let found = files.iter().filter(|p| p.is_file()).find_map(|p| {
        let content = std::fs::read_to_string(p).ok()?;
        content.contains("OAUTH_CLIENT_SECRET").then(|| parse_client(&content)).flatten()
    });
    if let (Some(c), Ok(mut cache)) = (&found, CLIENT_CACHE.lock()) {
        *cache = Some(c.clone());
    }
    found
}

fn access_token(creds: &serde_json::Value) -> Result<String, FetchError> {
    let now = chrono::Utc::now().timestamp();
    if let Some((tok, exp)) = TOKEN_CACHE.lock().ok().and_then(|c| c.clone()) {
        if exp - 60 > now {
            return Ok(tok);
        }
    }
    let file_token = creds.get("access_token").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
    let expiry = num(creds.get("expiry_date")).map(|ms| (ms / 1000.0) as i64);
    if let (Some(t), Some(exp)) = (file_token, expiry) {
        if exp - 60 > now {
            return Ok(t.to_string());
        }
    }
    let refresh = creds
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FetchError::Failed("Gemini token expired. Run `gemini` to sign in again.".into()))?;
    let (id, secret) = find_client().ok_or_else(|| {
        FetchError::Failed(
            "Gemini token expired and the Gemini CLI OAuth client was not found. Run `gemini` once to refresh.".into(),
        )
    })?;
    let resp = agent().post("https://oauth2.googleapis.com/token").send_form(&[
        ("client_id", id.as_str()),
        ("client_secret", secret.as_str()),
        ("refresh_token", refresh),
        ("grant_type", "refresh_token"),
    ]);
    let json = json_response(resp, "Gemini refresh token rejected. Run `gemini` to sign in again.")?;
    let tok = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FetchError::Failed("Gemini token refresh returned no access token.".into()))?
        .to_string();
    let exp = now + num(json.get("expires_in")).unwrap_or(3600.0) as i64;
    if let Ok(mut c) = TOKEN_CACHE.lock() {
        *c = Some((tok.clone(), exp));
    }
    Ok(tok)
}

pub fn fetch(_cfg: &SharedConfig) -> FetchResult {
    let dir = gemini_dir();
    if let Some(settings) = read_json_file(&dir.join("settings.json")) {
        let selected = settings
            .pointer("/security/auth/selectedType")
            .or_else(|| settings.get("selectedAuthType"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if selected == "gemini-api-key" || selected == "vertex-ai" {
            return Err(FetchError::NotConfigured(format!(
                "Gemini CLI uses {selected}; quota tracking needs the Google login (oauth-personal)."
            )));
        }
    }
    let path = dir.join("oauth_creds.json");
    let creds = read_json_file(&path).ok_or_else(|| {
        FetchError::NotConfigured(format!("No Gemini CLI login found ({}). Run `gemini` and sign in.", path.display()))
    })?;
    let token = access_token(&creds)?;
    let claims = creds.get("id_token").and_then(|v| v.as_str()).and_then(jwt_claims);
    let hint = "Gemini login expired. Run `gemini` to sign in again.";

    let ca = json_response(
        agent()
            .post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_string(r#"{"metadata":{"ideType":"GEMINI_CLI","pluginType":"GEMINI"}}"#),
        hint,
    )
    .ok();
    let project = ca.as_ref().and_then(|c| c.get("cloudaicompanionProject")).and_then(|p| {
        p.as_str()
            .map(String::from)
            .or_else(|| p.get("id").or_else(|| p.get("projectId")).and_then(|v| v.as_str()).map(String::from))
    });
    let tier = ca.as_ref().and_then(|c| c.pointer("/currentTier/id")).and_then(|v| v.as_str()).map(String::from);

    let body = match &project {
        Some(p) => serde_json::json!({ "project": p }).to_string(),
        None => "{}".into(),
    };
    let quota = json_response(
        agent()
            .post("https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota")
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_string(&body),
        hint,
    )?;

    // Lowest remaining fraction per tier (Pro / Flash / Flash Lite), as in GeminiStatusSnapshot.
    let mut tiers: [(&str, Option<(f64, Option<i64>)>); 3] = [("Pro (daily)", None), ("Flash (daily)", None), ("Flash Lite (daily)", None)];
    for b in quota.get("buckets").and_then(|v| v.as_array()).into_iter().flatten() {
        let (Some(model), Some(frac)) = (b.get("modelId").and_then(|v| v.as_str()), num(b.get("remainingFraction"))) else {
            continue;
        };
        let m = model.to_lowercase();
        let idx = if m.contains("flash-lite") {
            2
        } else if m.contains("flash") {
            1
        } else if m.contains("pro") {
            0
        } else {
            continue;
        };
        let reset = b.get("resetTime").and_then(|v| v.as_str()).and_then(parse_iso);
        if tiers[idx].1.map_or(true, |(f, _)| frac < f) {
            tiers[idx].1 = Some((frac, reset));
        }
    }

    let mut snap = ProviderSnapshot::new("gemini", "Gemini");
    snap.account = claims.as_ref().and_then(|c| c.get("email")).and_then(|v| v.as_str()).map(String::from);
    snap.plan = match tier.as_deref() {
        Some("free-tier") => Some("Free".into()),
        Some("standard-tier") => Some("Paid".into()),
        Some("legacy-tier") => Some("Legacy".into()),
        Some(other) => Some(capitalize(other)),
        None => None,
    };
    for (label, v) in tiers {
        if let Some((frac, reset)) = v {
            snap.windows.push(UsageWindow {
                label: label.into(),
                used_percent: (1.0 - frac) * 100.0,
                resets_at: reset,
                detail: None,
            });
        }
    }
    if snap.windows.is_empty() {
        snap.error = Some("Gemini returned no quota buckets for this account.".into());
    }
    Ok(snap)
}
