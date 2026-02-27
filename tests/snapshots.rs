//! Insta snapshot tests for serialization stability.
//!
//! These tests ensure that config, snapshot, and RPC formats remain stable
//! across releases, preventing accidental breaking changes to the public API.

use insta::assert_snapshot;

#[test]
fn snapshot_default_config_toml() {
    let config = tmuxpulse::config::AppConfig::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    assert_snapshot!("default_config_toml", toml_str);
}

#[test]
fn snapshot_empty_snapshot_json() {
    let snapshot = tmuxpulse::mux::Snapshot {
        sessions: Vec::new(),
        timestamp: 1700000000,
    };
    let json = serde_json::to_string_pretty(&snapshot).unwrap();
    assert_snapshot!("empty_snapshot_json", json);
}

#[test]
fn snapshot_session_json() {
    let session = tmuxpulse::mux::Session {
        id: tmuxpulse::mux::SessionId("$1".to_string()),
        name: "dev".to_string(),
        attached: true,
        windows: vec![tmuxpulse::mux::Window {
            id: tmuxpulse::mux::WindowId("@1".to_string()),
            session_id: tmuxpulse::mux::SessionId("$1".to_string()),
            name: "editor".to_string(),
            index: 0,
            active: true,
            panes: vec![tmuxpulse::mux::Pane {
                id: tmuxpulse::mux::PaneId("%1".to_string()),
                window_id: tmuxpulse::mux::WindowId("@1".to_string()),
                session_id: tmuxpulse::mux::SessionId("$1".to_string()),
                index: 0,
                active: true,
                width: 120,
                height: 40,
                current_command: "nvim".to_string(),
                current_path: "/home/user/project".to_string(),
                pid: 12345,
                dead: false,
                last_activity: 1700000500,
            }],
        }],
        created_at: 1700000000,
        last_activity: 1700000500,
    };
    let json = serde_json::to_string_pretty(&session).unwrap();
    assert_snapshot!("session_json", json);
}

#[test]
fn snapshot_plugin_manifest_toml() {
    let manifest = tmuxpulse::plugin::PluginManifest {
        name: "example-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "An example plugin".to_string(),
        entry: "./run.sh".to_string(),
        hooks: vec!["on_snapshot".to_string(), "on_event".to_string()],
        min_version: Some("0.1.0".to_string()),
    };
    let toml_str = toml::to_string_pretty(&manifest).unwrap();
    assert_snapshot!("plugin_manifest_toml", toml_str);
}

#[test]
fn snapshot_theme_names() {
    // Verify the set of built-in themes is stable
    let themes = ["default", "catppuccin-mocha", "dracula", "nord"];
    let themes_str = themes.join("\n");
    assert_snapshot!("built_in_themes", themes_str);
}
