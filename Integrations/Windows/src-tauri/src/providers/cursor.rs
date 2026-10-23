//! Cursor: access token from the Cursor app's `state.vscdb`
//! (`%APPDATA%\Cursor\User\globalStorage\state.vscdb`, key `cursorAuth/accessToken`) or the
//! `cursor-agent` login, turned into the `WorkosCursorSessionToken` cookie, then
//! `GET https://cursor.com/api/usage-summary` (see CursorAppAuth.swift / CursorStatusProbe).
//! A manual `cookieHeader` in config.json (cookieSource = manual) takes precedence.

use super::*;
use std::path::PathBuf;

fn db_candidates() -> Vec<PathBuf> {
    vec![
        crate::config::appdata_dir().join("Cursor").join("User").join("globalStorage").join("state.vscdb"),
        crate::config::home_dir().join(".config").join("Cursor").join("User").join("globalStorage").join("state.vscdb"),
    ]
}

fn read_db_token(path: &std::path::Path) -> Option<String> {
    use rusqlite::{Connection, OpenFlags};
    let uri = format!("file:{}?mode=ro", path.to_string_lossy().replace('\\', "/"));
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(&uri, flags).ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(2));
    let value: String = conn
        .query_row("SELECT value FROM ItemTable WHERE key = ?1 LIMIT 1", ["cursorAuth/accessToken"], |r| {
            r.get::<_, String>(0)
        })
        .ok()?;
    let v = value.trim().trim_matches('"').to_string();
    (!v.is_empty()).then_some(v)
}

fn agent_token() -> Option<String> {
    let candidates = [
        crate::config::home_dir().join(".config").join("cursor").join("auth.json"),
        crate::config::appdata_dir().join("cursor").join("auth.json"),
    ];
    candidates.iter().find_map(|p| {
        read_json_file(p)?
            .get("accessToken")?
            .as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

fn cookie_from_token(token: &str) -> Option<(String, Option<String>)> {
    let claims = jwt_claims(token)?;
    let sub = claims.get("sub")?.as_str()?;
    let user_id = sub.rsplit('|').next()?.to_string();
    if user_id.is_empty() || !user_id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
        return None;
    }
    let email = claims.get("email").and_then(|v| v.as_str()).map(String::from);
    Some((format!("WorkosCursorSessionToken={user_id}%3A%3A{token}"), email))
}

pub fn fetch(cfg: &SharedConfig) -> FetchResult {
    let manual = (cfg.provider_str("cursor", "cookieSource").as_deref() == Some("manual"))
        .then(|| cfg.provider_str("cursor", "cookieHeader"))
        .flatten();
    let (cookie, mut email) = if let Some(h) = manual {
        (h.trim_start_matches("Cookie:").trim().to_string(), None)
    } else {
        let token = db_candidates()
            .iter()
            .filter(|p| p.exists())
            .find_map(|p| read_db_token(p))
            .or_else(agent_token)
            .ok_or_else(|| {
                FetchError::NotConfigured(
                    "No Cursor login found. Sign in to the Cursor app, or paste a cookie header in Settings.".into(),
                )
            })?;
        cookie_from_token(&token)
            .ok_or_else(|| FetchError::Failed("Cursor access token could not be parsed.".into()))?
    };

    let hint = "Cursor session expired. Open Cursor and sign in again.";
    let summary = json_response(
        agent()
            .get("https://cursor.com/api/usage-summary")
            .set("Cookie", &cookie)
            .set("Accept", "application/json")
            .call(),
        hint,
    )?;
    if email.is_none() {
        if let Ok(me) = json_response(
            agent().get("https://cursor.com/api/auth/me").set("Cookie", &cookie).set("Accept", "application/json").call(),
            hint,
        ) {
            email = me.get("email").and_then(|v| v.as_str()).map(String::from);
        }
    }

    let mut snap = ProviderSnapshot::new("cursor", "Cursor");
    snap.account = email;
    snap.plan = summary.get("membershipType").and_then(|v| v.as_str()).map(capitalize);
    let reset = summary.get("billingCycleEnd").and_then(|v| v.as_str()).and_then(parse_iso);
    let indiv = summary.get("individualUsage");
    let plan = indiv.and_then(|i| i.get("plan"));
    let used_c = num(plan.and_then(|p| p.get("used")));
    let limit_c = num(plan.and_then(|p| p.get("limit")));
    let auto = num(plan.and_then(|p| p.get("autoPercentUsed")));
    let api = num(plan.and_then(|p| p.get("apiPercentUsed")));
    let total = num(plan.and_then(|p| p.get("totalPercentUsed")));
    let overall = indiv.and_then(|i| i.get("overall"));
    let pooled = summary.get("teamUsage").and_then(|t| t.get("pooled"));

    let pct = |u: Option<f64>, l: Option<f64>| match (u, l) {
        (Some(u), Some(l)) if l > 0.0 => Some(u / l * 100.0),
        _ => None,
    };
    let percent = total
        .or(match (auto, api) {
            (Some(a), Some(b)) => Some((a + b) / 2.0),
            (a, b) => a.or(b),
        })
        .or(pct(used_c, limit_c))
        .or(pct(num(overall.and_then(|o| o.get("used"))), num(overall.and_then(|o| o.get("limit")))))
        .or(pct(num(pooled.and_then(|o| o.get("used"))), num(pooled.and_then(|o| o.get("limit")))));

    let dollars = match (used_c, limit_c) {
        (Some(u), Some(l)) if l > 0.0 => Some(format!("${:.2} / ${:.2}", u / 100.0, l / 100.0)),
        _ => None,
    };
    snap.windows.push(UsageWindow {
        label: "Plan (billing cycle)".into(),
        used_percent: percent.unwrap_or(0.0),
        resets_at: reset,
        detail: dollars,
    });
    if let Some(a) = auto {
        snap.windows.push(UsageWindow { label: "Auto".into(), used_percent: a, resets_at: reset, detail: None });
    }
    if let Some(a) = api {
        snap.windows.push(UsageWindow { label: "API / named models".into(), used_percent: a, resets_at: reset, detail: None });
    }
    let od = indiv.and_then(|i| i.get("onDemand"));
    if let (Some(u), Some(l)) = (num(od.and_then(|o| o.get("used"))), num(od.and_then(|o| o.get("limit")))) {
        if l > 0.0 {
            snap.windows.push(UsageWindow {
                label: "On-demand".into(),
                used_percent: u / l * 100.0,
                resets_at: reset,
                detail: Some(format!("${:.2} / ${:.2}", u / 100.0, l / 100.0)),
            });
        }
    }
    Ok(snap)
}
