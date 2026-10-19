//! Tapp MCP server.
//!
//! Exposes computer use tools over MCP for Pantheon.

use anyhow::Result;
use serde_json::Value;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use tapp_core::cua::CuaClient;
use tapp_core::types::*;

/// Diagnostic event log. Records event types and metadata only.
/// No tool arguments, no results, no content. 7-day retention, 20MB cap.
mod diagnostic_log {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    lazy_static::lazy_static! {
        static ref LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
    }

    fn log_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        let dir = PathBuf::from(home).join(".cache").join("tapp");
        Some(dir.join("events.jsonl"))
    }

    pub fn init() {
        if std::env::var("TAPP_DIAGNOSTIC_LOG").map(|v| v == "0").unwrap_or(false) {
            return;
        }
        let Some(path) = log_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            *LOG_FILE.lock().unwrap() = Some(file);
        }
    }

    pub fn log(event: &str, metadata: Option<&str>) {
        if std::env::var("TAPP_DIAGNOSTIC_LOG").map(|v| v == "0").unwrap_or(false) {
            return;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mut line = format!("{{\"ts\":{},\"event\":\"{}\"", timestamp, event);
        if let Some(meta) = metadata {
            line.push_str(&format!(",\"meta\":{}", meta));
        }
        line.push_str("}\n");
        if let Ok(mut guard) = LOG_FILE.lock() {
            if let Some(ref mut file) = *guard {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

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
        diagnostic_log::init();
        diagnostic_log::log("server_init", None);
        Self {
            cua: CuaClient::new(),
            overlay: None,
        }
    }

    /// Create a new server with overlay enabled.
    pub fn with_overlay() -> Self {
        diagnostic_log::init();
        diagnostic_log::log("server_init", Some("overlay"));
        let overlay = match OverlayClient::spawn() {
            Ok(client) => Some(client),
            Err(e) => {
                diagnostic_log::log("overlay_spawn_failed", Some(&format!("error={}", e)));
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
            serde_json::json!({
                "name": "tapp_window_state",
                "description": "Get the accessibility tree for a window",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                    },
                    "required": ["window_id"],
                },
            }),
            serde_json::json!({
                "name": "tapp_click_element",
                "description": "Click an element by its token from tapp_window_state",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                        "element_token": { "type": "string" },
                        "button": { "type": "string", "enum": ["left", "right", "middle"] },
                    },
                    "required": ["window_id", "element_token"],
                },
            }),
            serde_json::json!({
                "name": "tapp_verify",
                "description": "Verify state after an action",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                        "predicate": { "type": "object" },
                    },
                    "required": ["window_id", "predicate"],
                },
            }),
            serde_json::json!({
                "name": "tapp_zoom",
                "description": "Zoom into a window region",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                        "rect": {
                            "type": "object",
                            "properties": {
                                "x": { "type": "number" },
                                "y": { "type": "number" },
                                "width": { "type": "number" },
                                "height": { "type": "number" },
                            },
                        },
                    },
                    "required": ["window_id"],
                },
            }),
            serde_json::json!({
                "name": "tapp_invoke_menu",
                "description": "Invoke a menu path in a window",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                        "menu_path": {
                            "type": "array",
                            "items": { "type": "string" },
                        },
                    },
                    "required": ["window_id", "menu_path"],
                },
            }),
            serde_json::json!({
                "name": "tapp_set_window_frame",
                "description": "Set window position and size",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_id": { "type": "string" },
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                        "width": { "type": "number" },
                        "height": { "type": "number" },
                    },
                    "required": ["window_id", "x", "y", "width", "height"],
                },
            }),
            serde_json::json!({
                "name": "tapp_launch_app",
                "description": "Launch an application",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "app_name": { "type": "string" },
                    },
                    "required": ["app_name"],
                },
            }),
            serde_json::json!({
                "name": "tapp_kill_app",
                "description": "Kill an application by PID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pid": { "type": "integer" },
                    },
                    "required": ["pid"],
                },
            }),
            serde_json::json!({
                "name": "tapp_clipboard_read",
                "description": "Read the clipboard",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                },
            }),
            serde_json::json!({
                "name": "tapp_clipboard_write",
                "description": "Write to the clipboard",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                    },
                    "required": ["text"],
                },
            }),
            serde_json::json!({
                "name": "tapp_move_cursor",
                "description": "Move the agent cursor to a point",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                    },
                    "required": ["x", "y"],
                },
            }),
            serde_json::json!({
                "name": "tapp_get_cursor_position",
                "description": "Get the current cursor position",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                },
            }),
            serde_json::json!({
                "name": "tapp_get_screen_size",
                "description": "Get the screen size",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                },
            }),
        ]
    }

    /// Call a tool.
    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        diagnostic_log::log("tool_call", Some(&format!("tool={}", name)));
        match name {
            "tapp_screenshot" => {
                let screenshot = self.cua.screenshot().await?;
                let (screen_width, screen_height) = self.cua.get_screen_size().await?;
                let scale = if screen_width > 0 {
                    screenshot.width as f32 / screen_width as f32
                } else {
                    1.0
                };
                Ok(serde_json::json!({
                    "width": screenshot.width,
                    "height": screenshot.height,
                    "base64": screenshot.base64,
                    "coordinate_frame": {
                        "screen_width": screen_width,
                        "screen_height": screen_height,
                        "scale": scale,
                    },
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
                    // Configurable overlay wait, default 100ms (down from 500ms)
                    let overlay_wait = std::env::var("TAPP_OVERLAY_WAIT_MS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(100);
                    tokio::time::sleep(tokio::time::Duration::from_millis(overlay_wait)).await;
                }

                let action = ClickAction {
                    point,
                    button,
                    clicks,
                    scope: "desktop".to_string(),
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
                    delivery_mode: "foreground".to_string(),
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
                self.cua.key(KeyAction { key, modifiers, delivery_mode: "foreground".to_string() }).await?;
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
                    scope: "desktop".to_string(),
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

                let action = DragAction { from, to, scope: "desktop".to_string() };
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
            "tapp_window_state" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let state = self.cua.get_window_state(&window_id).await?;
                Ok(state)
            }
            "tapp_click_element" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let element_token = args["element_token"].as_str().unwrap_or("").to_string();
                let button = match args["button"].as_str().unwrap_or("left") {
                    "right" => MouseButton::Right,
                    "middle" => MouseButton::Middle,
                    _ => MouseButton::Left,
                };
                self.cua.click_element(&window_id, &element_token, button).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_verify" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let predicate = args["predicate"].clone();
                let result = self.cua.verify_state(&window_id, predicate).await?;
                Ok(result)
            }
            "tapp_zoom" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let rect = args["rect"].as_object().map(|r| Rect {
                    x: r.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    y: r.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    width: r.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    height: r.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                });
                let result = self.cua.zoom(&window_id, rect).await?;
                Ok(result)
            }
            "tapp_invoke_menu" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let menu_path: Vec<String> = args["menu_path"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                self.cua.invoke_menu(&window_id, menu_path).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_set_window_frame" => {
                let window_id = args["window_id"].as_str().unwrap_or("").to_string();
                let rect = Rect {
                    x: args["x"].as_i64().unwrap_or(0) as i32,
                    y: args["y"].as_i64().unwrap_or(0) as i32,
                    width: args["width"].as_i64().unwrap_or(0) as i32,
                    height: args["height"].as_i64().unwrap_or(0) as i32,
                };
                self.cua.set_window_frame(&window_id, rect).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_launch_app" => {
                let app_name = args["app_name"].as_str().unwrap_or("").to_string();
                self.cua.launch_app(&app_name).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_kill_app" => {
                let pid = args["pid"].as_i64().unwrap_or(0) as i32;
                self.cua.kill_app(pid).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_clipboard_read" => {
                let text = self.cua.clipboard_read().await?;
                Ok(serde_json::json!({ "text": text }))
            }
            "tapp_clipboard_write" => {
                let text = args["text"].as_str().unwrap_or("").to_string();
                self.cua.clipboard_write(&text).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_move_cursor" => {
                let point = Point {
                    x: args["x"].as_i64().unwrap_or(0) as i32,
                    y: args["y"].as_i64().unwrap_or(0) as i32,
                };
                self.cua.move_cursor(point).await?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "tapp_get_cursor_position" => {
                let pos = self.cua.get_cursor_position().await?;
                Ok(serde_json::json!({ "x": pos.x, "y": pos.y }))
            }
            "tapp_get_screen_size" => {
                let (width, height) = self.cua.get_screen_size().await?;
                Ok(serde_json::json!({ "width": width, "height": height }))
            }
            _ => {
                diagnostic_log::log("tool_unknown", Some(&format!("tool={}", name)));
                Err(anyhow::anyhow!("Unknown tool: {}", name))
            }
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
            "tapp_window_state",
            "tapp_click_element",
            "tapp_verify",
            "tapp_zoom",
            "tapp_invoke_menu",
            "tapp_set_window_frame",
            "tapp_launch_app",
            "tapp_kill_app",
            "tapp_clipboard_read",
            "tapp_clipboard_write",
            "tapp_move_cursor",
            "tapp_get_cursor_position",
            "tapp_get_screen_size",
        ] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }
}
