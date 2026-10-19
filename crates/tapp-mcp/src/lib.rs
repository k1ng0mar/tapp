//! Tapp MCP server.
//!
//! Exposes computer use tools over MCP for Pantheon.

use anyhow::Result;
use serde_json::Value;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use tapp_core::cua::CuaClient;
use tapp_core::types::*;

/// Overlay client that communicates with the cursor overlay process.
struct OverlayClient {
    child: Child,
}

impl OverlayClient {
    /// Spawn the overlay process.
    fn spawn() -> Result<Self> {
        let child = Command::new("tapp-overlay")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        Ok(Self { child })
    }

    /// Send an overlay event.
    fn send_event(&mut self, event: &OverlayEvent) -> Result<()> {
        let stdin = self.child.stdin.as_mut().unwrap();
        let event_json = serde_json::to_string(event)?;
        writeln!(stdin, "{}", event_json)?;
        stdin.flush()?;
        Ok(())
    }

    /// Show cursor at a point with optional label.
    fn show_cursor(&mut self, point: Point, label: Option<String>) -> Result<()> {
        self.send_event(&OverlayEvent::ShowCursor { point, label })
    }

    /// Animate cursor from one point to another.
    fn animate_to(&mut self, from: Point, to: Point, duration_ms: u64) -> Result<()> {
        self.send_event(&OverlayEvent::AnimateTo {
            from,
            to,
            duration_ms,
        })
    }

    /// Hide cursor.
    fn hide_cursor(&mut self) -> Result<()> {
        self.send_event(&OverlayEvent::HideCursor)
    }
}

/// Parse a key modifier name, erroring on unknown values instead of
/// silently substituting something the caller didn't ask for.
fn parse_modifier(s: &str) -> Result<Modifier> {
    match s {
        "ctrl" => Ok(Modifier::Ctrl),
        "alt" => Ok(Modifier::Alt),
        "shift" => Ok(Modifier::Shift),
        "meta" => Ok(Modifier::Meta),
        other => Err(anyhow::anyhow!("unknown modifier: {other}")),
    }
}

/// Tapp MCP server.
pub struct TappServer {
    cua: CuaClient,
    overlay: Option<OverlayClient>,
}

impl Default for TappServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TappServer {
    /// Create a new server.
    pub fn new() -> Self {
        Self {
            cua: CuaClient::new(),
            overlay: None,
        }
    }

    /// Create a new server with overlay enabled.
    pub fn with_overlay() -> Self {
        let overlay = match OverlayClient::spawn() {
            Ok(client) => Some(client),
            Err(e) => {
                eprintln!("tapp: overlay disabled, could not spawn tapp-overlay: {e}");
                None
            }
        };
        Self {
            cua: CuaClient::new(),
            overlay,
        }
    }

    /// List available tools.
    pub fn list_tools() -> Vec<Value> {
        vec![
            serde_json::json!({
                "name": "tapp_screenshot",
                "description": "Take a screenshot of the screen",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                },
            }),
            serde_json::json!({
                "name": "tapp_click",
                "description": "Click at a point on screen",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                        "button": { "type": "string", "enum": ["left", "right", "middle"] },
                        "clicks": { "type": "number" },
                    },
                    "required": ["x", "y"],
                },
            }),
            serde_json::json!({
                "name": "tapp_type",
                "description": "Type text",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                    },
                    "required": ["text"],
                },
            }),
            serde_json::json!({
                "name": "tapp_key",
                "description": "Press a key",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "modifiers": {
                            "type": "array",
                            "items": { "type": "string", "enum": ["ctrl", "alt", "shift", "meta"] },
                        },
                    },
                    "required": ["key"],
                },
            }),
            serde_json::json!({
                "name": "tapp_scroll",
                "description": "Scroll at a point on screen",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                        "delta_x": { "type": "number" },
                        "delta_y": { "type": "number" },
                    },
                    "required": ["x", "y"],
                },
            }),
            serde_json::json!({
                "name": "tapp_drag",
                "description": "Drag from one point to another",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "from_x": { "type": "number" },
                        "from_y": { "type": "number" },
                        "to_x": { "type": "number" },
                        "to_y": { "type": "number" },
                    },
                    "required": ["from_x", "from_y", "to_x", "to_y"],
                },
            }),
            serde_json::json!({
                "name": "tapp_list_windows",
                "description": "List all open windows",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                },
            }),
            serde_json::json!({
                "name": "tapp_bring_to_front",
                "description": "Bring a window to front by window_id",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "integer" },
                    },
                    "required": ["window_id"],
                },
            }),
        ]
    }

    /// Call a tool.
    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        match name {
            "tapp_screenshot" => {
                let screenshot = self.cua.screenshot().await?;
                Ok(serde_json::json!({
                    "width": screenshot.width,
                    "height": screenshot.height,
                    "base64": screenshot.base64,
                }))
            }
            "tapp_click" => {
                let point = Point {
                    x: args["x"].as_i64().unwrap_or(0) as i32,
                    y: args["y"].as_i64().unwrap_or(0) as i32,
                };
                let button = match args["button"].as_str().unwrap_or("left") {
                    "right" => MouseButton::Right,
                    "middle" => MouseButton::Middle,
                    _ => MouseButton::Left,
                };
                let clicks = args["clicks"].as_u64().unwrap_or(1) as u32;

                // Show overlay before clicking
                if let Some(ref mut overlay) = self.overlay {
                    let _ = overlay.show_cursor(point, Some("click".to_string()));
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                let action = ClickAction {
                    point,
                    button,
                    clicks,
                };
                self.cua.click(action).await?;

                // Hide overlay after clicking
                if let Some(ref mut overlay) = self.overlay {
                    let _ = overlay.hide_cursor();
                }

                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_type" => {
                let action = TypeAction {
                    text: args["text"].as_str().unwrap_or("").to_string(),
                };
                self.cua.type_text(action).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_key" => {
                let key = args["key"].as_str().unwrap_or("").to_string();
                if key.is_empty() {
                    return Err(anyhow::anyhow!("tapp_key requires a non-empty key"));
                }
                let modifiers: Vec<Modifier> = args["modifiers"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(parse_modifier)
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default();
                self.cua.key(KeyAction { key, modifiers }).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_scroll" => {
                let action = ScrollAction {
                    point: Point {
                        x: args["x"].as_i64().unwrap_or(0) as i32,
                        y: args["y"].as_i64().unwrap_or(0) as i32,
                    },
                    delta_x: args["delta_x"].as_i64().unwrap_or(0) as i32,
                    delta_y: args["delta_y"].as_i64().unwrap_or(0) as i32,
                };
                self.cua.scroll(action).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_drag" => {
                let from = Point {
                    x: args["from_x"].as_i64().unwrap_or(0) as i32,
                    y: args["from_y"].as_i64().unwrap_or(0) as i32,
                };
                let to = Point {
                    x: args["to_x"].as_i64().unwrap_or(0) as i32,
                    y: args["to_y"].as_i64().unwrap_or(0) as i32,
                };

                // Show overlay before dragging
                if let Some(ref mut overlay) = self.overlay {
                    let _ = overlay.animate_to(from, to, 500);
                }

                let action = DragAction { from, to };
                self.cua.drag(action).await?;

                // Hide overlay after dragging
                if let Some(ref mut overlay) = self.overlay {
                    let _ = overlay.hide_cursor();
                }

                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_list_windows" => {
                let windows = self.cua.list_windows().await?;
                Ok(windows)
            }
            "tapp_bring_to_front" => {
                let window_id = args["window_id"].as_i64().unwrap_or(0);
                self.cua.bring_to_front(window_id).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modifier_accepts_known_names() {
        assert!(matches!(parse_modifier("ctrl"), Ok(Modifier::Ctrl)));
        assert!(matches!(parse_modifier("alt"), Ok(Modifier::Alt)));
        assert!(matches!(parse_modifier("shift"), Ok(Modifier::Shift)));
        assert!(matches!(parse_modifier("meta"), Ok(Modifier::Meta)));
    }

    #[test]
    fn parse_modifier_rejects_unknown_names() {
        assert!(parse_modifier("super").is_err());
        assert!(parse_modifier("").is_err());
        assert!(parse_modifier("CTRL").is_err());
    }

    #[test]
    fn tool_names_are_stable_and_documented() {
        let tools = TappServer::list_tools();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name")?.as_str())
            .collect();
        for expected in [
            "tapp_screenshot",
            "tapp_click",
            "tapp_type",
            "tapp_key",
            "tapp_scroll",
            "tapp_drag",
            "tapp_list_windows",
            "tapp_bring_to_front",
        ] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }
}
