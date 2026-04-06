//! Independent UI configuration with persistent storage at ~/.macrdp/config-ui.toml

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// UI-specific configuration. All fields have sensible defaults so
/// `UiConfig::default()` is always usable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    // ── Server ──────────────────────────────────────────────
    pub port: u16,
    pub frame_rate: u32,
    pub bitrate_mbps: u32,
    pub encoder: String,
    pub chroma_mode: String,
    pub bind_address: String,
    pub max_connections: u32,
    pub idle_timeout_secs: u64,

    // ── Auth ────────────────────────────────────────────────
    pub username: String,
    pub password: String,

    // ── Display ─────────────────────────────────────────────
    pub hidpi_scale: u32,
    pub show_cursor: bool,

    // ── Application ─────────────────────────────────────────
    pub log_level: String,
    pub theme: String,
    pub autostart: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            port: 3389,
            frame_rate: 60,
            bitrate_mbps: 50,
            encoder: "auto".to_string(),
            chroma_mode: "avc420".to_string(),
            bind_address: "0.0.0.0".to_string(),
            max_connections: 3,
            idle_timeout_secs: 1800,
            username: "macrdp".to_string(),
            password: String::new(),
            hidpi_scale: 2,
            show_cursor: false,
            log_level: "warn".to_string(),
            theme: "system".to_string(),
            autostart: false,
        }
    }
}

/// Returns the config file path: `~/.macrdp/config-ui.toml`
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".macrdp")
        .join("config-ui.toml")
}

impl UiConfig {
    /// Load from `~/.macrdp/config-ui.toml`, creating defaults if the file
    /// does not exist or is unparseable.
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        if !path.exists() {
            let cfg = Self::default();
            // Try to persist the defaults so the user has a file to edit.
            let _ = cfg.save();
            return Ok(cfg);
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("parse config: {e}"))
    }

    /// Serialize to TOML and write to `~/.macrdp/config-ui.toml`.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create config dir: {e}"))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("serialize config: {e}"))?;
        std::fs::write(&path, content).map_err(|e| format!("write config: {e}"))
    }

    /// Update a single field by key name from a JSON value.
    /// Returns `restart_required`: `false` for hot-updatable fields
    /// (frame_rate, bitrate_mbps, log_level, theme, autostart), `true` for
    /// everything else.
    pub fn set_field(
        &mut self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<bool, String> {
        match key {
            "port" => {
                self.port = value
                    .as_u64()
                    .ok_or("port must be a number")?
                    as u16;
            }
            "frame_rate" => {
                self.frame_rate = value
                    .as_u64()
                    .ok_or("frame_rate must be a number")?
                    as u32;
            }
            "bitrate_mbps" => {
                self.bitrate_mbps = value
                    .as_u64()
                    .ok_or("bitrate_mbps must be a number")?
                    as u32;
            }
            "encoder" => {
                self.encoder = value
                    .as_str()
                    .ok_or("encoder must be a string")?
                    .to_string();
            }
            "chroma_mode" => {
                self.chroma_mode = value
                    .as_str()
                    .ok_or("chroma_mode must be a string")?
                    .to_string();
            }
            "bind_address" => {
                self.bind_address = value
                    .as_str()
                    .ok_or("bind_address must be a string")?
                    .to_string();
            }
            "max_connections" => {
                self.max_connections = value
                    .as_u64()
                    .ok_or("max_connections must be a number")?
                    as u32;
            }
            "idle_timeout_secs" => {
                self.idle_timeout_secs = value
                    .as_u64()
                    .ok_or("idle_timeout_secs must be a number")?;
            }
            "username" => {
                self.username = value
                    .as_str()
                    .ok_or("username must be a string")?
                    .to_string();
            }
            "password" => {
                self.password = value
                    .as_str()
                    .ok_or("password must be a string")?
                    .to_string();
            }
            "hidpi_scale" => {
                self.hidpi_scale = value
                    .as_u64()
                    .ok_or("hidpi_scale must be a number")?
                    as u32;
            }
            "show_cursor" => {
                self.show_cursor = value
                    .as_bool()
                    .ok_or("show_cursor must be a boolean")?;
            }
            "log_level" => {
                self.log_level = value
                    .as_str()
                    .ok_or("log_level must be a string")?
                    .to_string();
            }
            "theme" => {
                self.theme = value
                    .as_str()
                    .ok_or("theme must be a string")?
                    .to_string();
            }
            "autostart" => {
                self.autostart = value
                    .as_bool()
                    .ok_or("autostart must be a boolean")?;
            }
            _ => return Err(format!("unknown config key: {key}")),
        }

        // Hot-updatable fields do NOT require a restart.
        let restart_required = !matches!(
            key,
            "frame_rate" | "bitrate_mbps" | "log_level" | "theme" | "autostart"
        );
        Ok(restart_required)
    }

    /// Convert to the core library's `ServerConfig` used to start the server.
    pub fn to_server_config(&self) -> macrdp_core::ServerConfig {
        macrdp_core::ServerConfig {
            port: self.port,
            frame_rate: self.frame_rate,
            width: 0,  // auto-detect
            height: 0, // auto-detect
            username: Some(self.username.clone()),
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
            cert_path: None,
            key_path: None,
            idle_timeout_secs: self.idle_timeout_secs,
            log_level: Some(self.log_level.clone()),
            quality: None,
            encoder: Some(self.encoder.clone()),
            chroma_mode: Some(self.chroma_mode.clone()),
            hidpi_scale: Some(self.hidpi_scale),
            bitrate_mbps: Some(self.bitrate_mbps),
        }
    }
}
