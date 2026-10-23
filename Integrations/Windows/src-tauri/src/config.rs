//! Shared CodexBar `config.json` (same format as the macOS/Linux app and CLI) plus
//! Windows-only UI preferences stored separately so the shared file stays portable.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn appdata_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join("AppData").join("Roaming"))
}

pub fn local_appdata_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join("AppData").join("Local"))
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Resolution order mirrors docs/configuration.md:
/// CODEXBAR_CONFIG, absolute XDG_CONFIG_HOME, ~/.config/codexbar, legacy ~/.codexbar.
pub fn config_path() -> PathBuf {
    if let Some(p) = non_empty_env("CODEXBAR_CONFIG") {
        return PathBuf::from(p);
    }
    if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
        let p = PathBuf::from(&xdg);
        if p.is_absolute() {
            return p.join("codexbar").join("config.json");
        }
    }
    let modern = home_dir().join(".config").join("codexbar").join("config.json");
    let legacy = home_dir().join(".codexbar").join("config.json");
    if !modern.exists() && legacy.exists() {
        return legacy;
    }
    modern
}

/// Raw config kept as JSON so unknown fields/providers survive our writes untouched.
#[derive(Clone, Debug)]
pub struct SharedConfig {
    pub path: PathBuf,
    pub root: Value,
    pub error: Option<String>,
}

impl SharedConfig {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => match serde_json::from_str::<Value>(&text) {
                Ok(v) if v.is_object() => Self { path, root: v, error: None },
                Ok(_) => Self { path, root: default_root(), error: Some("config.json is not a JSON object".into()) },
                Err(e) => Self { path, root: default_root(), error: Some(format!("config.json is invalid: {e}")) },
            },
            _ => Self { path, root: default_root(), error: None },
        }
    }

    pub fn provider(&self, id: &str) -> Option<&Map<String, Value>> {
        self.root
            .get("providers")?
            .as_array()?
            .iter()
            .filter_map(|p| p.as_object())
            .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id))
    }

    pub fn provider_str(&self, id: &str, key: &str) -> Option<String> {
        self.provider(id)?
            .get(key)?
            .as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn provider_enabled(&self, id: &str) -> Option<bool> {
        self.provider(id)?.get("enabled")?.as_bool()
    }

    /// Sets one provider field, appending a provider entry if needed. `None` writes JSON null.
    pub fn set_provider_field(&mut self, id: &str, key: &str, value: Value) {
        let root = self.root.as_object_mut().expect("root object");
        root.entry("version").or_insert(Value::from(1));
        let providers = root.entry("providers").or_insert_with(|| Value::Array(vec![]));
        if !providers.is_array() {
            *providers = Value::Array(vec![]);
        }
        let arr = providers.as_array_mut().unwrap();
        if let Some(p) = arr
            .iter_mut()
            .filter_map(|p| p.as_object_mut())
            .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id))
        {
            p.insert(key.to_string(), value);
            return;
        }
        let mut m = Map::new();
        m.insert("id".into(), Value::from(id));
        m.insert(key.to_string(), value);
        arr.push(Value::Object(m));
    }

    /// Atomic write (temp file + rename). Refuses to overwrite a file we could not parse.
    pub fn save(&self) -> Result<(), String> {
        if self.error.is_some() {
            return Err("Refusing to overwrite an invalid config.json; fix it first.".into());
        }
        write_atomic(&self.path, &serde_json::to_vec_pretty(&self.root).map_err(|e| e.to_string())?)
    }
}

fn default_root() -> Value {
    serde_json::json!({ "version": 1, "providers": [] })
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Windows-only UI preferences (%APPDATA%\CodexBar\windows-settings.json).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    /// Minutes between background refreshes; 0 = manual only.
    pub refresh_minutes: u32,
    pub notifications: bool,
    pub warn_threshold: f64,
    pub critical_threshold: f64,
    /// Set once so first launch can enable start-with-Windows by default.
    pub autostart_initialized: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_minutes: 5,
            notifications: true,
            warn_threshold: 80.0,
            critical_threshold: 95.0,
            autostart_initialized: false,
        }
    }
}

pub fn settings_path() -> PathBuf {
    appdata_dir().join("CodexBar").join("windows-settings.json")
}

impl AppSettings {
    pub fn load() -> Self {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        write_atomic(&settings_path(), &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
    }
}
