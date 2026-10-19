//! Tapp MCP binary.

use anyhow::Result;
use std::io::{self, BufRead, Write};
use tapp_mcp::TappServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut server = TappServer::with_overlay();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: serde_json::Value = serde_json::from_str(&line)?;
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

        if id.is_none() {
            continue
        }

        let response = match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                    },
                    "serverInfo": {
                        "name": "tapp",
                        "version": "0.1.0",
                    },
                },
            }),
            "tools/list" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": TappServer::list_tools(),
                },
            }),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or_default();
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_default();

                match server.call_tool(name, args).await {
                    Ok(result) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string(&result)?,
                                },
                            ],
                        },
                    }),
                    Err(e) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32000,
                            "message": e.to_string(),
                        },
                    }),
                }
            }
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                },
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}
