//! Daemon RPC server for TmuxPulse.
//!
//! Runs as a background process, serving a JSON-RPC API over a Unix domain socket.
//! This enables AI agents, scripts, and external tools to query tmux state
//! and send commands without needing to parse tmux output directly.
//!
//! Protocol: Newline-delimited JSON (NDJSON) over Unix socket.
//!
//! Methods:
//!   pulse.ping           - health check
//!   pulse.snapshot       - get current tmux snapshot
//!   pulse.sessions       - list session names and IDs
//!   pulse.capture        - capture pane output
//!   pulse.send_keys      - send keys to a pane
//!   pulse.kill_session   - kill a session
//!   pulse.version        - get TmuxPulse version info

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::mux::tmux::TmuxClient;
use crate::mux::{PaneId, SessionId, Snapshot};

/// JSON-RPC request
#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: serde_json::Value,
    /// Request ID for correlation
    pub id: serde_json::Value,
}

/// JSON-RPC response
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: serde_json::Value,
}

/// JSON-RPC error
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError { code, message }),
            id,
        }
    }
}

/// Shared daemon state
pub struct DaemonState {
    pub client: TmuxClient,
    pub last_snapshot: RwLock<Option<Snapshot>>,
    pub auth_token: Option<String>,
    pub start_time: std::time::Instant,
}

/// Daemon server configuration
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub auth_token: Option<String>,
}

impl DaemonConfig {
    /// Default socket path: $XDG_RUNTIME_DIR/tmuxpulse.sock or /tmp/tmuxpulse-$UID.sock
    pub fn default_socket_path() -> PathBuf {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            return PathBuf::from(runtime_dir).join("tmuxpulse.sock");
        }

        // Fallback: /tmp/tmuxpulse-UID.sock
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/tmuxpulse-{}.sock", uid))
    }
}

/// Start the daemon server
pub async fn start_daemon(config: DaemonConfig, client: TmuxClient) -> Result<()> {
    let socket_path = &config.socket_path;

    // Clean up stale socket file
    if socket_path.exists() {
        // Check if another daemon is running
        if UnixStream::connect(socket_path).await.is_ok() {
            anyhow::bail!(
                "another daemon is already running on {}",
                socket_path.display()
            );
        }
        std::fs::remove_file(socket_path)
            .with_context(|| format!("failed to remove stale socket: {}", socket_path.display()))?;
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("failed to bind socket: {}", socket_path.display()))?;

    // Set socket permissions to owner-only (0600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(socket_path, perms).ok();
    }

    let state = Arc::new(DaemonState {
        client,
        last_snapshot: RwLock::new(None),
        auth_token: config.auth_token,
        start_time: std::time::Instant::now(),
    });

    info!("daemon listening on {}", socket_path.display());

    // Spawn a background task to periodically refresh the snapshot
    let refresh_state = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            match refresh_state.client.snapshot().await {
                Ok(snapshot) => {
                    let mut lock = refresh_state.last_snapshot.write().await;
                    *lock = Some(snapshot);
                }
                Err(e) => {
                    warn!("daemon snapshot refresh error: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state).await {
                        debug!("connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("accept error: {}", e);
            }
        }
    }
}

/// Handle a single client connection
async fn handle_connection(stream: UnixStream, state: Arc<DaemonState>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(request) => handle_request(request, &state).await,
            Err(e) => RpcResponse::error(
                serde_json::Value::Null,
                -32700,
                format!("parse error: {}", e),
            ),
        };

        let json = serde_json::to_string(&response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Route a JSON-RPC request to the appropriate handler
async fn handle_request(req: RpcRequest, state: &DaemonState) -> RpcResponse {
    // Auth check
    if let Some(ref token) = state.auth_token {
        if token != "auto" {
            let provided = req
                .params
                .get("auth_token")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if provided != token {
                return RpcResponse::error(req.id, -32000, "unauthorized".to_string());
            }
        }
    }

    match req.method.as_str() {
        "pulse.ping" => RpcResponse::success(req.id, serde_json::json!({"pong": true})),

        "pulse.version" => RpcResponse::success(
            req.id,
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_secs": state.start_time.elapsed().as_secs(),
            }),
        ),

        "pulse.snapshot" => {
            let lock = state.last_snapshot.read().await;
            match &*lock {
                Some(snapshot) => match serde_json::to_value(snapshot) {
                    Ok(val) => RpcResponse::success(req.id, val),
                    Err(e) => {
                        RpcResponse::error(req.id, -32603, format!("serialization error: {}", e))
                    }
                },
                None => {
                    // No cached snapshot, fetch live
                    drop(lock);
                    match state.client.snapshot().await {
                        Ok(snapshot) => match serde_json::to_value(&snapshot) {
                            Ok(val) => RpcResponse::success(req.id, val),
                            Err(e) => RpcResponse::error(
                                req.id,
                                -32603,
                                format!("serialization error: {}", e),
                            ),
                        },
                        Err(e) => RpcResponse::error(req.id, -32603, format!("tmux error: {}", e)),
                    }
                }
            }
        }

        "pulse.sessions" => {
            let lock = state.last_snapshot.read().await;
            let sessions: Vec<serde_json::Value> = match &*lock {
                Some(snapshot) => snapshot
                    .sessions
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "id": s.id.0,
                            "name": s.name,
                            "attached": s.attached,
                            "windows": s.windows.len(),
                            "panes": s.windows.iter().map(|w| w.panes.len()).sum::<usize>(),
                        })
                    })
                    .collect(),
                None => Vec::new(),
            };
            RpcResponse::success(req.id, serde_json::json!(sessions))
        }

        "pulse.capture" => {
            let pane_id = match req.params.get("pane_id").and_then(|v| v.as_str()) {
                Some(id) => PaneId(id.to_string()),
                None => {
                    return RpcResponse::error(
                        req.id,
                        -32602,
                        "missing required param: pane_id".to_string(),
                    );
                }
            };
            let lines = req
                .params
                .get("lines")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;

            match state.client.capture_pane(&pane_id, lines).await {
                Ok(content) => RpcResponse::success(
                    req.id,
                    serde_json::json!({
                        "pane_id": pane_id.0,
                        "content": content,
                        "lines": content.lines().count(),
                    }),
                ),
                Err(e) => RpcResponse::error(req.id, -32603, format!("capture error: {}", e)),
            }
        }

        "pulse.send_keys" => {
            let pane_id = match req.params.get("pane_id").and_then(|v| v.as_str()) {
                Some(id) => PaneId(id.to_string()),
                None => {
                    return RpcResponse::error(
                        req.id,
                        -32602,
                        "missing required param: pane_id".to_string(),
                    );
                }
            };
            let keys: Vec<String> = match req.params.get("keys") {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                Some(serde_json::Value::String(s)) => vec![s.clone()],
                _ => {
                    return RpcResponse::error(
                        req.id,
                        -32602,
                        "missing required param: keys".to_string(),
                    );
                }
            };

            match state.client.send_keys(&pane_id, &keys).await {
                Ok(()) => RpcResponse::success(req.id, serde_json::json!({"sent": true})),
                Err(e) => RpcResponse::error(req.id, -32603, format!("send_keys error: {}", e)),
            }
        }

        "pulse.kill_session" => {
            let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                Some(id) => SessionId(id.to_string()),
                None => {
                    return RpcResponse::error(
                        req.id,
                        -32602,
                        "missing required param: session_id".to_string(),
                    );
                }
            };

            match state.client.kill_session(&session_id).await {
                Ok(()) => RpcResponse::success(req.id, serde_json::json!({"killed": true})),
                Err(e) => RpcResponse::error(req.id, -32603, format!("kill_session error: {}", e)),
            }
        }

        _ => RpcResponse::error(req.id, -32601, format!("method not found: {}", req.method)),
    }
}

/// Check if a daemon is currently running
pub async fn is_daemon_running(socket_path: &Path) -> bool {
    if !socket_path.exists() {
        return false;
    }
    UnixStream::connect(socket_path).await.is_ok()
}

/// Send a single RPC request to the daemon and get the response
pub async fn rpc_call(
    socket_path: &Path,
    method: &str,
    params: serde_json::Value,
) -> Result<RpcResponse> {
    let stream = UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let (reader, mut writer) = stream.into_split();

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });

    let json = serde_json::to_string(&request)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut lines = BufReader::new(reader).lines();
    let line = lines
        .next_line()
        .await?
        .context("daemon closed connection")?;

    let response: RpcResponse = serde_json::from_str(&line)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_response_success_serialization() {
        let resp = RpcResponse::success(serde_json::json!(1), serde_json::json!({"pong": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn rpc_response_error_serialization() {
        let resp = RpcResponse::error(serde_json::json!(1), -32601, "method not found".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn parse_rpc_request() {
        let json = r#"{"jsonrpc":"2.0","method":"pulse.ping","params":{},"id":1}"#;
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "pulse.ping");
        assert_eq!(req.id, serde_json::json!(1));
    }

    #[test]
    fn parse_rpc_request_with_params() {
        let json = r#"{"jsonrpc":"2.0","method":"pulse.capture","params":{"pane_id":"%1","lines":100},"id":2}"#;
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "pulse.capture");
        assert_eq!(req.params["pane_id"], "%1");
        assert_eq!(req.params["lines"], 100);
    }

    #[test]
    fn default_socket_path_not_empty() {
        let path = DaemonConfig::default_socket_path();
        assert!(!path.as_os_str().is_empty());
        assert!(path.to_string_lossy().contains("tmuxpulse"));
    }
}
