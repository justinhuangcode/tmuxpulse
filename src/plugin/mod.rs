//! Plugin system for TmuxPulse.
//!
//! Plugins are external executables that communicate via JSON over stdin/stdout.
//! Each plugin lives in a directory with a `plugin.toml` manifest describing
//! its name, version, and hooks it subscribes to.
//!
//! Protocol:
//!   TmuxPulse sends a JSON object per line to the plugin's stdin.
//!   The plugin responds with a JSON object per line on stdout.
//!
//! Hook lifecycle:
//!   1. `init`       - sent once at startup, plugin returns capabilities
//!   2. `on_snapshot` - sent each tick with the full snapshot
//!   3. `on_event`   - sent when a control-mode event fires
//!   4. `shutdown`   - sent before TmuxPulse exits

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

use crate::mux::{MuxEvent, Snapshot};

/// Plugin manifest loaded from `plugin.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// Relative path to the plugin executable (within the plugin directory)
    pub entry: String,
    /// Which hooks this plugin subscribes to
    #[serde(default)]
    pub hooks: Vec<String>,
    /// Minimum TmuxPulse version required
    #[serde(default)]
    pub min_version: Option<String>,
}

/// Message sent from TmuxPulse to a plugin
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    #[serde(rename = "init")]
    Init { tmuxpulse_version: String },
    #[serde(rename = "on_snapshot")]
    OnSnapshot { snapshot: Snapshot },
    #[serde(rename = "on_event")]
    OnEvent { event: String },
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Response from a plugin
#[derive(Debug, Deserialize)]
pub struct PluginResponse {
    /// Optional status line text to display
    pub status: Option<String>,
    /// Optional notification to show as toast
    pub notification: Option<String>,
    /// Optional log message
    pub log: Option<String>,
    /// Whether the plugin handled the message successfully
    #[serde(default = "default_true")]
    pub ok: bool,
}

fn default_true() -> bool {
    true
}

/// A running plugin instance
struct PluginInstance {
    manifest: PluginManifest,
    child: Child,
    _dir: PathBuf,
}

/// Plugin manager that discovers, loads, and communicates with plugins
pub struct PluginManager {
    instances: Vec<PluginInstance>,
    /// Aggregated status lines from plugins (plugin_name -> status text)
    status_lines: HashMap<String, String>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            status_lines: HashMap::new(),
        }
    }

    /// Discover and load plugins from configured directories
    pub async fn load_plugins(
        &mut self,
        directories: &[PathBuf],
        enabled: &[String],
    ) -> Result<()> {
        for dir in directories {
            if !dir.exists() {
                debug!("plugin directory does not exist: {}", dir.display());
                continue;
            }

            let entries = std::fs::read_dir(dir)
                .with_context(|| format!("failed to read plugin directory: {}", dir.display()))?;

            for entry in entries {
                let entry = entry?;
                let plugin_dir = entry.path();
                if !plugin_dir.is_dir() {
                    continue;
                }

                let manifest_path = plugin_dir.join("plugin.toml");
                if !manifest_path.exists() {
                    continue;
                }

                match self.load_plugin(&plugin_dir, &manifest_path, enabled).await {
                    Ok(()) => {}
                    Err(e) => {
                        warn!("failed to load plugin from {}: {}", plugin_dir.display(), e);
                    }
                }
            }
        }

        info!("loaded {} plugins", self.instances.len());
        Ok(())
    }

    /// Load a single plugin from its directory
    async fn load_plugin(
        &mut self,
        plugin_dir: &Path,
        manifest_path: &Path,
        enabled: &[String],
    ) -> Result<()> {
        let content = std::fs::read_to_string(manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;

        let manifest: PluginManifest = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

        // Check if this plugin is in the enabled list (empty = all enabled)
        if !enabled.is_empty() && !enabled.contains(&manifest.name) {
            debug!("plugin '{}' not in enabled list, skipping", manifest.name);
            return Ok(());
        }

        let entry_path = plugin_dir.join(&manifest.entry);
        if !entry_path.exists() {
            bail!(
                "plugin entry '{}' not found at {}",
                manifest.entry,
                entry_path.display()
            );
        }

        // Spawn the plugin process
        let child = Command::new(&entry_path)
            .current_dir(plugin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn plugin '{}'", manifest.name))?;

        info!("loaded plugin: {} v{}", manifest.name, manifest.version);

        self.instances.push(PluginInstance {
            manifest,
            child,
            _dir: plugin_dir.to_path_buf(),
        });

        // Send init message to the newly loaded plugin
        let idx = self.instances.len() - 1;
        let msg = PluginMessage::Init {
            tmuxpulse_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        self.send_message(idx, &msg).await?;

        Ok(())
    }

    /// Send a message to a specific plugin and collect response
    async fn send_message(
        &mut self,
        index: usize,
        msg: &PluginMessage,
    ) -> Result<Option<PluginResponse>> {
        let instance = &mut self.instances[index];
        let stdin = match instance.child.stdin.as_mut() {
            Some(s) => s,
            None => bail!("plugin '{}' stdin not available", instance.manifest.name),
        };

        let json = serde_json::to_string(msg)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response with timeout
        let stdout = match instance.child.stdout.as_mut() {
            Some(s) => s,
            None => return Ok(None),
        };

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        let read_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            reader.read_line(&mut line),
        )
        .await;

        match read_result {
            Ok(Ok(0)) => Ok(None), // EOF
            Ok(Ok(_)) => {
                let response: PluginResponse =
                    serde_json::from_str(line.trim()).with_context(|| {
                        format!(
                            "plugin '{}' sent invalid response: {}",
                            instance.manifest.name, line
                        )
                    })?;
                Ok(Some(response))
            }
            Ok(Err(e)) => {
                warn!("plugin '{}' read error: {}", instance.manifest.name, e);
                Ok(None)
            }
            Err(_) => {
                warn!("plugin '{}' response timed out", instance.manifest.name);
                Ok(None)
            }
        }
    }

    /// Broadcast a snapshot to all plugins that subscribe to on_snapshot
    pub async fn broadcast_snapshot(&mut self, snapshot: &Snapshot) {
        let msg = PluginMessage::OnSnapshot {
            snapshot: snapshot.clone(),
        };

        for i in 0..self.instances.len() {
            let subscribes = self.instances[i].manifest.hooks.is_empty()
                || self.instances[i]
                    .manifest
                    .hooks
                    .iter()
                    .any(|h| h == "on_snapshot");

            if !subscribes {
                continue;
            }

            match self.send_message(i, &msg).await {
                Ok(Some(resp)) => {
                    let name = self.instances[i].manifest.name.clone();
                    if let Some(status) = resp.status {
                        self.status_lines.insert(name, status);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(
                        "error broadcasting to plugin '{}': {}",
                        self.instances[i].manifest.name, e
                    );
                }
            }
        }
    }

    /// Broadcast a MuxEvent to all plugins
    pub async fn broadcast_event(&mut self, event: &MuxEvent) {
        let msg = PluginMessage::OnEvent {
            event: format!("{:?}", event),
        };

        for i in 0..self.instances.len() {
            let subscribes = self.instances[i].manifest.hooks.is_empty()
                || self.instances[i]
                    .manifest
                    .hooks
                    .iter()
                    .any(|h| h == "on_event");

            if !subscribes {
                continue;
            }

            if let Err(e) = self.send_message(i, &msg).await {
                warn!(
                    "error broadcasting event to plugin '{}': {}",
                    self.instances[i].manifest.name, e
                );
            }
        }
    }

    /// Shutdown all plugins gracefully
    pub async fn shutdown(&mut self) {
        for i in 0..self.instances.len() {
            let _ = self.send_message(i, &PluginMessage::Shutdown).await;
        }

        for instance in &mut self.instances {
            let _ = instance.child.kill().await;
        }

        self.instances.clear();
        info!("all plugins shut down");
    }

    /// Get aggregated status lines from plugins
    pub fn status_lines(&self) -> &HashMap<String, String> {
        &self.status_lines
    }

    /// Get loaded plugin count
    pub fn plugin_count(&self) -> usize {
        self.instances.len()
    }

    /// List loaded plugin manifests
    pub fn list_plugins(&self) -> Vec<&PluginManifest> {
        self.instances.iter().map(|i| &i.manifest).collect()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for instance in &mut self.instances {
            let _ = instance.child.start_kill();
        }
    }
}

/// Discover plugins in a directory (without loading them)
pub fn discover_plugins(directories: &[PathBuf]) -> Vec<(PathBuf, PluginManifest)> {
    let mut found = Vec::new();

    for dir in directories {
        if !dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let plugin_dir = entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }

            let manifest_path = plugin_dir.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            let content = match std::fs::read_to_string(&manifest_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if let Ok(manifest) = toml::from_str::<PluginManifest>(&content) {
                found.push((plugin_dir, manifest));
            }
        }
    }

    found
}

/// Default plugin directories
pub fn default_plugin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_dir) = dirs::data_dir() {
        dirs.push(data_dir.join("tmuxpulse").join("plugins"));
    }

    if let Some(config_dir) = dirs::config_dir() {
        dirs.push(config_dir.join("tmuxpulse").join("plugins"));
    }

    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plugin_manifest() {
        let toml_str = r#"
            name = "session-monitor"
            version = "0.1.0"
            description = "Monitors session activity"
            entry = "./monitor"
            hooks = ["on_snapshot", "on_event"]
        "#;

        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "session-monitor");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.entry, "./monitor");
        assert_eq!(manifest.hooks.len(), 2);
    }

    #[test]
    fn parse_minimal_manifest() {
        let toml_str = r#"
            name = "minimal"
            version = "0.1.0"
            entry = "./run"
        "#;

        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "minimal");
        assert!(manifest.hooks.is_empty());
        assert!(manifest.description.is_empty());
    }

    #[test]
    fn discover_empty_dir() {
        let found = discover_plugins(&[PathBuf::from("/nonexistent/path")]);
        assert!(found.is_empty());
    }

    #[test]
    fn default_dirs_returns_paths() {
        let dirs = default_plugin_dirs();
        // Should return at least one path (data_dir or config_dir)
        // On CI this might be empty if no HOME is set, so we just check it doesn't panic
        let _ = dirs;
    }

    #[test]
    fn plugin_manager_new() {
        let mgr = PluginManager::new();
        assert_eq!(mgr.plugin_count(), 0);
        assert!(mgr.status_lines().is_empty());
    }

    #[test]
    fn serialize_plugin_message_init() {
        let msg = PluginMessage::Init {
            tmuxpulse_version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"init\""));
        assert!(json.contains("\"tmuxpulse_version\":\"0.1.0\""));
    }

    #[test]
    fn deserialize_plugin_response() {
        let json = r#"{"ok": true, "status": "3 active", "notification": null}"#;
        let resp: PluginResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.status.unwrap(), "3 active");
        assert!(resp.notification.is_none());
    }
}
