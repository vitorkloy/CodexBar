// CodexBar for Windows: system-tray app showing AI coding usage limits and reset countdowns.
// Based on CodexBar by Peter Steinberger (MIT): https://github.com/steipete/CodexBar
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod config;
mod providers;

use config::{AppSettings, SharedConfig};
use providers::{ProviderSnapshot, PROVIDERS};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_notification::NotificationExt as _;
use tauri_plugin_opener::OpenerExt as _;

const TRAY_ID: &str = "main";
const PANEL_W: f64 = 360.0;
const PANEL_H: f64 = 540.0;

struct Shared {
    snapshots: Mutex<Vec<ProviderSnapshot>>,
    last_refresh: Mutex<Option<i64>>,
    refreshing: Mutex<bool>,
    settings: Mutex<AppSettings>,
    notified: Mutex<HashSet<String>>,
    wake: (Mutex<bool>, Condvar),
    panel_opened_at: Mutex<Option<Instant>>,
}

impl Shared {
    fn wake(&self) {
        let (lock, cv) = &self.wake;
        *lock.lock().unwrap() = true;
        cv.notify_all();
    }
}

type St<'a> = tauri::State<'a, Arc<Shared>>;

fn enabled_ids(cfg: &SharedConfig) -> Vec<&'static str> {
    PROVIDERS.iter().filter(|p| cfg.provider_enabled(p.id).unwrap_or(true)).map(|p| p.id).collect()
}

// ---------- refresh loop ----------

fn refresh_all(app: &AppHandle, shared: &Shared) {
    *shared.refreshing.lock().unwrap() = true;
    let _ = app.emit("usage-updated", ());
    let cfg = SharedConfig::load();
    let ids = enabled_ids(&cfg);
    // Fetch providers concurrently; threads exist only for the duration of a refresh.
    let results: Vec<ProviderSnapshot> = std::thread::scope(|s| {
        let handles: Vec<_> = ids.iter().map(|id| s.spawn(|| providers::fetch(id, &cfg))).collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    *shared.snapshots.lock().unwrap() = results;
    *shared.last_refresh.lock().unwrap() = Some(chrono::Utc::now().timestamp());
    *shared.refreshing.lock().unwrap() = false;
    update_tray(app, shared);
    maybe_notify(app, shared);
    let _ = app.emit("usage-updated", ());
}

fn poll_loop(app: AppHandle, shared: Arc<Shared>) {
    loop {
        refresh_all(&app, &shared);
        let minutes = shared.settings.lock().unwrap().refresh_minutes;
        let (lock, cv) = &shared.wake;
        let mut woken = lock.lock().unwrap();
        if minutes == 0 {
            while !*woken {
                woken = cv.wait(woken).unwrap();
            }
        } else {
            let deadline = Instant::now() + Duration::from_secs(minutes as u64 * 60);
            while !*woken {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                woken = cv.wait_timeout(woken, deadline - now).unwrap().0;
            }
        }
        *woken = false;
    }
}

// ---------- tray ----------

fn fmt_reset(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else { return String::new() };
    let now = Local::now();
    if dt.date_naive() == now.date_naive() {
        dt.format("%H:%M").to_string()
    } else if (dt - now).num_days() < 6 {
        dt.format("%a %H:%M").to_string()
    } else {
        dt.format("%d %b").to_string()
    }
}

fn headline(s: &ProviderSnapshot) -> Option<&providers::UsageWindow> {
    s.windows.iter().find(|w| w.detail.as_deref() != Some("Unlimited / not metered"))
}

fn tray_bar_icon(max_used: Option<f64>, warn: f64, crit: f64) -> Image<'static> {
    let base = tauri::include_image!("icons/32x32.png");
    let (w, h) = (base.width() as usize, base.height() as usize);
    let mut rgba = base.rgba().to_vec();
    if let Some(used) = max_used {
        let color: [u8; 3] = if used >= crit {
            [0xef, 0x44, 0x44]
        } else if used >= warn {
            [0xf5, 0x9e, 0x0b]
        } else {
            [0x22, 0xc5, 0x5e]
        };
        let bar_h = (h / 7).max(3);
        let filled = ((used.clamp(0.0, 100.0) / 100.0) * w as f64).round() as usize;
        for y in h - bar_h..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let px: [u8; 4] = if x < filled.max(1) { [color[0], color[1], color[2], 255] } else { [40, 40, 46, 230] };
                rgba[i..i + 4].copy_from_slice(&px);
            }
        }
    }
    Image::new_owned(rgba, w as u32, h as u32)
}

fn update_tray(app: &AppHandle, shared: &Shared) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let snaps = shared.snapshots.lock().unwrap().clone();
    let settings = shared.settings.lock().unwrap().clone();

    let mut tip_parts = vec![];
    let mut max_used: Option<f64> = None;
    for s in &snaps {
        if let Some(w) = headline(s) {
            tip_parts.push(format!("{} {:.0}%", s.name, w.used_percent));
            max_used = Some(max_used.map_or(w.used_percent, |m: f64| m.max(w.used_percent)));
        }
    }
    let mut tip = if tip_parts.is_empty() { "CodexBar".to_string() } else { format!("CodexBar\n{}", tip_parts.join(" · ")) };
    if tip.chars().count() > 120 {
        tip = tip.chars().take(119).collect::<String>() + "…";
    }
    let _ = tray.set_tooltip(Some(tip));
    let _ = tray.set_icon(Some(tray_bar_icon(max_used, settings.warn_threshold, settings.critical_threshold)));
    if let Ok(menu) = build_menu(app, &snaps) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu(app: &AppHandle, snaps: &[ProviderSnapshot]) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for s in snaps {
        let text = match (headline(s), &s.error) {
            (Some(w), _) => {
                let reset = w.resets_at.map(|t| format!(" · resets {}", fmt_reset(t))).unwrap_or_default();
                format!("{}: {:.0}% {}{}", s.name, w.used_percent, w.label.to_lowercase(), reset)
            }
            (None, _) if s.not_configured => format!("{}: not set up", s.name),
            (None, Some(_)) => format!("{}: error (open panel)", s.name),
            _ => format!("{}: no data", s.name),
        };
        menu.append(&MenuItem::with_id(app, format!("p:{}", s.id), text, true, None::<&str>)?)?;
    }
    if !snaps.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "open", "Open panel", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit CodexBar", true, None::<&str>)?)?;
    Ok(menu)
}

// ---------- notifications ----------

fn maybe_notify(app: &AppHandle, shared: &Shared) {
    let settings = shared.settings.lock().unwrap().clone();
    if !settings.notifications {
        return;
    }
    let snaps = shared.snapshots.lock().unwrap().clone();
    let mut notified = shared.notified.lock().unwrap();
    for s in &snaps {
        for w in &s.windows {
            let level = if w.used_percent >= settings.critical_threshold {
                "critical"
            } else if w.used_percent >= settings.warn_threshold {
                "warn"
            } else {
                continue;
            };
            let key = format!("{}|{}|{}|{}", s.id, w.label, w.resets_at.unwrap_or(0), level);
            if !notified.insert(key) {
                continue;
            }
            let reset = w.resets_at.map(|t| format!(" Resets {}.", fmt_reset(t))).unwrap_or_default();
            let _ = app
                .notification()
                .builder()
                .title(format!("{} {} at {:.0}%", s.name, w.label, w.used_percent))
                .body(format!(
                    "{}{}",
                    if level == "critical" { "Almost out of quota." } else { "Getting close to the limit." },
                    reset
                ))
                .show();
        }
    }
}

// ---------- windows ----------

fn toggle_panel(app: &AppHandle, anchor: Option<(f64, f64, f64, f64)>) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.close();
        return;
    }
    open_panel(app, anchor);
}

fn open_panel(app: &AppHandle, anchor: Option<(f64, f64, f64, f64)>) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.set_focus();
        return;
    }
    let Ok(win) = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("index.html".into()))
        .title("CodexBar")
        .inner_size(PANEL_W, PANEL_H)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()
    else {
        return;
    };
    let scale = win.scale_factor().unwrap_or(1.0);
    let (pw, ph) = (PANEL_W * scale, PANEL_H * scale);
    let monitor = anchor
        .and_then(|(ax, ay, _, _)| win.monitor_from_point(ax, ay).ok().flatten())
        .or_else(|| win.primary_monitor().ok().flatten());
    let (mx, my, mw, mh) = monitor
        .as_ref()
        .map(|m| (m.position().x as f64, m.position().y as f64, m.size().width as f64, m.size().height as f64))
        .unwrap_or((0.0, 0.0, 1920.0, 1080.0));
    let (x, y) = match anchor {
        Some((ax, ay, aw, ah)) => {
            let x = (ax + aw / 2.0 - pw / 2.0).clamp(mx + 8.0, mx + mw - pw - 8.0);
            let y = if ay > my + mh / 2.0 { ay - ph - 8.0 } else { ay + ah + 8.0 };
            (x, y.clamp(my, my + mh - ph))
        }
        None => (mx + mw - pw - 16.0, my + mh - ph - 64.0),
    };
    let _ = win.set_position(PhysicalPosition::new(x, y));
    *app.state::<Arc<Shared>>().panel_opened_at.lock().unwrap() = Some(Instant::now());
    let w2 = win.clone();
    let app2 = app.clone();
    win.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            // Destroy (not hide) so the WebView2 processes exit and idle RAM stays low.
            let opened = *app2.state::<Arc<Shared>>().panel_opened_at.lock().unwrap();
            if opened.map_or(true, |t| t.elapsed() > Duration::from_millis(400)) {
                let _ = w2.close();
            }
        }
    });
    let _ = win.show();
    let _ = win.set_focus();
    // Windows may refuse to give focus (foreground lock), in which case no Focused(false)
    // ever arrives. Watchdog: close the panel once it has been unfocused for ~2s.
    // Runs only while the panel exists.
    let w3 = win.clone();
    let _ = std::thread::Builder::new().name("panel-watchdog".into()).spawn(move || {
        let mut unfocused = 0u8;
        std::thread::sleep(Duration::from_millis(1500));
        loop {
            match w3.is_focused() {
                Err(_) => break, // window destroyed
                Ok(true) => unfocused = 0,
                Ok(false) => {
                    unfocused += 1;
                    if unfocused >= 4 {
                        let _ = w3.close();
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

fn open_settings_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    if let Some(p) = app.get_webview_window("panel") {
        let _ = p.close();
    }
    if let Ok(w) = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("CodexBar Settings")
        .inner_size(560.0, 720.0)
        .min_inner_size(480.0, 520.0)
        .center()
        .build()
    {
        let _ = w.set_focus();
    }
}

// ---------- commands ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatePayload {
    providers: Vec<ProviderSnapshot>,
    last_refresh: Option<i64>,
    refreshing: bool,
    refresh_minutes: u32,
    warn_threshold: f64,
    critical_threshold: f64,
}

#[tauri::command]
fn get_state(state: St) -> StatePayload {
    let s = state.settings.lock().unwrap().clone();
    StatePayload {
        providers: state.snapshots.lock().unwrap().clone(),
        last_refresh: *state.last_refresh.lock().unwrap(),
        refreshing: *state.refreshing.lock().unwrap(),
        refresh_minutes: s.refresh_minutes,
        warn_threshold: s.warn_threshold,
        critical_threshold: s.critical_threshold,
    }
}

#[tauri::command]
fn refresh_now(state: St) {
    state.wake();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderToggle {
    id: &'static str,
    name: &'static str,
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPayload {
    settings: AppSettings,
    autostart: bool,
    providers: Vec<ProviderToggle>,
    config_path: String,
    config_error: Option<String>,
    copilot_has_token: bool,
    copilot_host: Option<String>,
    cursor_manual: bool,
    version: String,
}

#[tauri::command]
fn get_settings(app: AppHandle, state: St) -> SettingsPayload {
    let cfg = SharedConfig::load();
    SettingsPayload {
        settings: state.settings.lock().unwrap().clone(),
        autostart: autostart::is_enabled(),
        providers: PROVIDERS
            .iter()
            .map(|p| ProviderToggle { id: p.id, name: p.name, enabled: cfg.provider_enabled(p.id).unwrap_or(true) })
            .collect(),
        config_path: cfg.path.display().to_string(),
        config_error: cfg.error.clone(),
        copilot_has_token: cfg.provider_str("copilot", "apiKey").is_some(),
        copilot_host: cfg.provider_str("copilot", "enterpriseHost"),
        cursor_manual: cfg.provider_str("cursor", "cookieSource").as_deref() == Some("manual")
            && cfg.provider_str("cursor", "cookieHeader").is_some(),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn save_settings(
    state: St,
    settings: AppSettings,
    autostart: bool,
    enabled: HashMap<String, bool>,
) -> Result<(), String> {
    let mut s = settings;
    s.refresh_minutes = s.refresh_minutes.min(240);
    s.warn_threshold = s.warn_threshold.clamp(1.0, 100.0);
    s.critical_threshold = s.critical_threshold.clamp(s.warn_threshold, 100.0);
    s.autostart_initialized = true;
    s.save()?;
    *state.settings.lock().unwrap() = s;

    if autostart != autostart::is_enabled() {
        autostart::set(autostart)?;
    }

    let mut cfg = SharedConfig::load();
    let mut changed = false;
    for p in PROVIDERS {
        if let Some(&on) = enabled.get(p.id) {
            if cfg.provider_enabled(p.id).unwrap_or(true) != on {
                cfg.set_provider_field(p.id, "enabled", serde_json::Value::Bool(on));
                changed = true;
            }
        }
    }
    if changed {
        cfg.save()?;
    }
    state.wake();
    Ok(())
}

#[tauri::command]
fn set_cursor_cookie(state: St, header: String) -> Result<(), String> {
    let mut cfg = SharedConfig::load();
    let h = header.trim();
    if h.is_empty() {
        cfg.set_provider_field("cursor", "cookieSource", "auto".into());
        cfg.set_provider_field("cursor", "cookieHeader", serde_json::Value::Null);
    } else {
        cfg.set_provider_field("cursor", "cookieSource", "manual".into());
        cfg.set_provider_field("cursor", "cookieHeader", h.into());
    }
    cfg.save()?;
    state.wake();
    Ok(())
}

#[tauri::command]
fn set_copilot_token(state: St, token: String, host: Option<String>) -> Result<(), String> {
    let mut cfg = SharedConfig::load();
    let t = token.trim();
    cfg.set_provider_field("copilot", "apiKey", if t.is_empty() { serde_json::Value::Null } else { t.into() });
    if let Some(h) = host {
        let h = h.trim();
        cfg.set_provider_field("copilot", "enterpriseHost", if h.is_empty() { serde_json::Value::Null } else { h.into() });
    }
    cfg.save()?;
    state.wake();
    Ok(())
}

#[tauri::command]
async fn copilot_sign_in(app: AppHandle) -> Result<providers::copilot::DeviceCode, String> {
    let host = providers::copilot::normalize_host(SharedConfig::load().provider_str("copilot", "enterpriseHost"));
    let h2 = host.clone();
    let code = tauri::async_runtime::spawn_blocking(move || providers::copilot::device_start(&h2))
        .await
        .map_err(|e| e.to_string())??;
    let _ = app.opener().open_url(code.verification_uri.clone(), None::<&str>);
    let c2 = code.clone();
    std::thread::spawn(move || {
        let result = providers::copilot::device_poll(&host, &c2).and_then(|token| {
            let mut cfg = SharedConfig::load();
            cfg.set_provider_field("copilot", "apiKey", token.into());
            cfg.save()
        });
        let msg = match &result {
            Ok(()) => serde_json::json!({ "ok": true, "message": "Signed in to GitHub Copilot." }),
            Err(e) => serde_json::json!({ "ok": false, "message": e }),
        };
        let _ = app.emit("copilot-signin", msg);
        app.state::<Arc<Shared>>().wake();
    });
    Ok(code)
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}

#[tauri::command]
fn open_config_folder(app: AppHandle) {
    let p = config::config_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
        let _ = app.opener().open_path(dir.display().to_string(), None::<&str>);
    }
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) {
    if url.starts_with("https://") {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

#[tauri::command]
fn close_panel(app: AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.close();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn main() {
    let settings = AppSettings::load();
    let shared = Arc::new(Shared {
        snapshots: Mutex::new(vec![]),
        last_refresh: Mutex::new(None),
        refreshing: Mutex::new(false),
        settings: Mutex::new(settings),
        notified: Mutex::new(HashSet::new()),
        wake: (Mutex::new(false), Condvar::new()),
        panel_opened_at: Mutex::new(None),
    });

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| open_panel(app, None)))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![
            get_state,
            refresh_now,
            get_settings,
            save_settings,
            set_cursor_cookie,
            set_copilot_token,
            copilot_sign_in,
            open_settings,
            open_config_folder,
            open_url,
            close_panel,
            quit
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let menu = build_menu(&handle, &[])?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_bar_icon(None, 80.0, 95.0))
                .tooltip("CodexBar")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                        let scale = 1.0;
                        let pos = rect.position.to_physical::<f64>(scale);
                        let size = rect.size.to_physical::<f64>(scale);
                        toggle_panel(tray.app_handle(), Some((pos.x, pos.y, size.width, size.height)));
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => open_panel(app, None),
                    "refresh" => app.state::<Arc<Shared>>().wake(),
                    "settings" => open_settings_window(app),
                    "quit" => app.exit(0),
                    id if id.starts_with("p:") => open_panel(app, None),
                    _ => {}
                })
                .build(app)?;

            // First launch: start with Windows by default (can be turned off in Settings).
            {
                let st = handle.state::<Arc<Shared>>();
                let mut s = st.settings.lock().unwrap();
                if !s.autostart_initialized {
                    let _ = autostart::set(true);
                    s.autostart_initialized = true;
                    let _ = s.save();
                }
            }

            let shared = shared.clone();
            std::thread::Builder::new()
                .name("codexbar-poll".into())
                .spawn(move || poll_loop(handle, shared))?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building CodexBar");

    app.run(|_app, event| {
        if let RunEvent::ExitRequested { api, code, .. } = event {
            // Closing the last window must not quit the tray app; only explicit Quit does.
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
