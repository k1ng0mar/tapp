//! Shared types for Tapp.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A point on screen.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A rectangle on screen.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Screenshot result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub base64: String,
}

/// Click action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickAction {
    pub point: Point,
    pub button: MouseButton,
    pub clicks: u32,
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_scope() -> String {
    "desktop".to_string()
}

/// Mouse button.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Type action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAction {
    pub text: String,
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: String,
}

fn default_delivery_mode() -> String {
    "foreground".to_string()
}

/// Key action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAction {
    pub key: String,
    pub modifiers: Vec<Modifier>,
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: String,
}

/// Key modifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

/// Scroll action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollAction {
    pub point: Point,
    pub delta_x: i32,
    pub delta_y: i32,
    #[serde(default = "default_scope")]
    pub scope: String,
}

/// Drag action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragAction {
    pub from: Point,
    pub to: Point,
    #[serde(default = "default_scope")]
    pub scope: String,
}


/// Window state with accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub window_id: String,
    pub title: String,
    pub elements: Vec<Element>,
}

/// An element in the accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub token: String,
    pub role: String,
    pub label: String,
    pub frame: Rect,
    pub parent: Option<usize>,
}

/// Verification result after an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub success: bool,
    pub message: String,
    pub state_before: Option<Value>,
    pub state_after: Option<Value>,
}

/// Coordinate frame information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateFrame {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

/// Refusal code from cua-driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refusal {
    pub code: String,
    pub message: String,
}

/// Overlay event for the cursor overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayEvent {
    /// Show cursor at point with optional label.
    ShowCursor {
        point: Point,
        label: Option<String>,
    },
    /// Hide cursor.
    HideCursor,
    /// Animate cursor to point.
    AnimateTo {
        from: Point,
        to: Point,
        duration_ms: u64,
    },
}
