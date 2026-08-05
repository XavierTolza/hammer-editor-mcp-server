//! MCP JSON-RPC protocol handler — reads from stdin, writes to stdout.

use crate::server::{JsonRpcRequest, McpServer};
use serde_json::{json, Value};
use std::sync::Mutex;
use tokio::io::{stdin, stdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

pub async fn run_loop(server_url: String) -> anyhow::Result<()> {
    let server = Mutex::new(McpServer::new(server_url));
    let stdin = BufReader::new(stdin());
    let mut lines = stdin.lines();
    let mut stdout = stdout();

    info!(
        "Hammer Editor MCP Server starting (protocol v{})",
        crate::models::PROTOCOL_VERSION
    );

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse JSON-RPC: {}", e);
                continue;
            }
        };

        if req.method == "notifications/initialized" {
            continue;
        }

        info!("→ {}", req.method);

        let resp = match req.method.as_str() {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": { "name": "hammer-editor-mcp-server", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": { "tools": {} }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": { "tools": McpServer::tool_definitions() }
            }),
            "tools/call" => {
                let params = req.params.as_ref();
                let name = params.and_then(|p| p["name"].as_str());
                let args = params
                    .and_then(|p| p.get("arguments").cloned())
                    .unwrap_or(Value::Null);

                match name {
                    Some(n) => {
                        let call_result = {
                            let mut guard = server.lock().unwrap();
                            guard.handle_call(n, &args).await
                        };
                        match call_result {
                            Ok(result) => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": req.id,
                                    "result": { "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Serialization error: {}", e))}] }
                                })
                            }
                            Err(e) => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": req.id,
                                    "result": { "content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true }
                                })
                            }
                        }
                    }
                    None => json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "error": { "code": -32602, "message": "Missing tool name" }
                    }),
                }
            }
            "ping" => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {}
            }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "error": { "code": -32601, "message": format!("Unknown method: {}", req.method) }
            }),
        };

        let resp_line = format!("{}\n", serde_json::to_string(&resp)?);
        stdout.write_all(resp_line.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}
