//! Cua client for Tapp.
//!
//! Talks to the cua-driver daemon via its CLI.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use tokio::process::Command;

use crate::types::*;

/// Client for the cua-driver daemon.
pub struct CuaClient {
    binary: PathBuf,
}

impl Default for CuaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CuaClient {
    /// Create a new client with the default binary path.
    ///
    /// The path is taken from the `TAPP_CUA_DRIVER` environment variable when
    /// set, otherwise falls back to `cua-driver` on `PATH`.
    pub fn new() -> Self {
        let binary = std::env::var_os("TAPP_CUA_DRIVER")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cua-driver"));
        Self { binary }
    }

    /// Create a new client with a custom binary path.
    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
        }
    }

    /// Call a cua-driver tool via CLI.
    async fn call(&self, tool: &str, args: Value) -> Result<Value> {
        let args_str = serde_json::to_string(&args)?;
        let output = Command::new(&self.binary)
            .args(["call", tool, &args_str])
            .output()
            .await
            .context("failed to run cua-driver")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cua-driver call {} failed: {}", tool, stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: Value = serde_json::from_str(&stdout)?;
        Ok(response)
    }

    /// Take a screenshot of the full desktop.
    pub async fn screenshot(&self) -> Result<Screenshot> {
        let response = self.call("get_desktop_state", serde_json::json!({})).await?;
        let width = response["screenshot_width"].as_u64().unwrap_or(0) as u32;
        let height = response["screenshot_height"].as_u64().unwrap_or(0) as u32;
        let base64 = response["screenshot_png_b64"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(Screenshot {
            width,
            height,
            base64,
        })
    }

    /// Click at a point on screen.
    pub async fn click(&self, action: ClickAction) -> Result<()> {
        self.call(
            "click",
            serde_json::json!({
                "x": action.point.x,
                "y": action.point.y,
                "button": match action.button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                },
                "count": action.clicks,
                "scope": action.scope,
            }),
        )
        .await?;
        Ok(())
    }

    /// Click an element by token.
    pub async fn click_element(&self, window_id: &str, element_token: &str, button: MouseButton) -> Result<()> {
        self.call(
            "click",
            serde_json::json!({
                "window_id": window_id,
                "element_token": element_token,
                "button": match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                },
                "scope": "element",
            }),
        )
        .await?;
        Ok(())
    }

    /// Type text.
    pub async fn type_text(&self, action: TypeAction) -> Result<()> {
        self.call(
            "type_text",
            serde_json::json!({
                "text": action.text,
                "delivery_mode": action.delivery_mode,
            }),
        )
        .await?;
        Ok(())
    }

    /// Press a key.
    pub async fn key(&self, action: KeyAction) -> Result<()> {
        self.call(
            "press_key",
            serde_json::json!({
                "key": action.key,
                "modifiers": action.modifiers,
                "delivery_mode": action.delivery_mode,
            }),
        )
        .await?;
        Ok(())
    }

    /// Scroll at a point.
    pub async fn scroll(&self, action: ScrollAction) -> Result<()> {
        self.call(
            "scroll",
            serde_json::json!({
                "x": action.point.x,
                "y": action.point.y,
                "delta_x": action.delta_x,
                "delta_y": action.delta_y,
                "scope": action.scope,
            }),
        )
        .await?;
        Ok(())
    }

    /// Drag from one point to another.
    pub async fn drag(&self, action: DragAction) -> Result<()> {
        self.call(
            "drag",
            serde_json::json!({
                "from_x": action.from.x,
                "from_y": action.from.y,
                "to_x": action.to.x,
                "to_y": action.to.y,
                "scope": action.scope,
            }),
        )
        .await?;
        Ok(())
    }

    /// Get the list of windows.
    pub async fn list_windows(&self) -> Result<Value> {
        self.call("list_windows", serde_json::json!({})).await
    }

    /// Get the accessibility tree for a window.
    pub async fn get_window_state(&self, window_id: &str) -> Result<Value> {
        self.call(
            "get_window_state",
            serde_json::json!({ "window_id": window_id }),
        )
        .await
    }

    /// Get the screen size.
    pub async fn get_screen_size(&self) -> Result<(i32, i32)> {
        let response = self.call("get_screen_size", serde_json::json!({})).await?;
        let width = response["width"].as_i64().unwrap_or(0) as i32;
        let height = response["height"].as_i64().unwrap_or(0) as i32;
        Ok((width, height))
    }

    /// Move the agent cursor to a point.
    pub async fn move_cursor(&self, point: Point) -> Result<()> {
        self.call(
            "move_cursor",
            serde_json::json!({
                "x": point.x,
                "y": point.y,
            }),
        )
        .await?;
        Ok(())
    }

    /// Get the current cursor position.
    pub async fn get_cursor_position(&self) -> Result<Point> {
        let response = self.call("get_cursor_position", serde_json::json!({})).await?;
        let x = response["x"].as_i64().unwrap_or(0) as i32;
        let y = response["y"].as_i64().unwrap_or(0) as i32;
        Ok(Point { x, y })
    }

    /// Launch an app.
    pub async fn launch_app(&self, app_name: &str) -> Result<()> {
        self.call(
            "launch_app",
            serde_json::json!({ "app_name": app_name }),
        )
        .await?;
        Ok(())
    }

    /// Kill an app by pid.
    pub async fn kill_app(&self, pid: i32) -> Result<()> {
        self.call("kill_app", serde_json::json!({ "pid": pid })).await?;
        Ok(())
    }

    /// Verify state after an action.
    pub async fn verify_state(&self, window_id: &str, predicate: Value) -> Result<Value> {
        self.call(
            "verify_state",
            serde_json::json!({ "window_id": window_id, "predicate": predicate }),
        )
        .await
    }

    /// Zoom into a window region.
    pub async fn zoom(&self, window_id: &str, rect: Option<Rect>) -> Result<Value> {
        let mut args = serde_json::json!({ "window_id": window_id });
        if let Some(r) = rect {
            args["rect"] = serde_json::json!({
                "x": r.x,
                "y": r.y,
                "width": r.width,
                "height": r.height,
            });
        }
        self.call("zoom", args).await
    }

    /// Invoke a menu path.
    pub async fn invoke_menu(&self, window_id: &str, menu_path: Vec<String>) -> Result<()> {
        self.call(
            "invoke_menu",
            serde_json::json!({ "window_id": window_id, "menu_path": menu_path }),
        )
        .await?;
        Ok(())
    }

    /// Set window frame (position and size).
    pub async fn set_window_frame(&self, window_id: &str, rect: Rect) -> Result<()> {
        self.call(
            "set_window_frame",
            serde_json::json!({
                "window_id": window_id,
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
            }),
        )
        .await?;
        Ok(())
    }

    /// Bring a window to front.
    pub async fn bring_to_front(&self, window_id: i64) -> Result<()> {
        self.call(
            "bring_to_front",
            serde_json::json!({ "window_id": window_id }),
        )
        .await?;
        Ok(())
    }

    /// Read the clipboard.
    pub async fn clipboard_read(&self) -> Result<String> {
        let response = self.call("clipboard_read", serde_json::json!({})).await?;
        Ok(response["text"].as_str().unwrap_or("").to_string())
    }

    /// Write to the clipboard.
    pub async fn clipboard_write(&self, text: &str) -> Result<()> {
        self.call(
            "clipboard_write",
            serde_json::json!({ "text": text }),
        )
        .await?;
        Ok(())
    }
}
