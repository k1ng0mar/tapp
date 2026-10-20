//! Shared types for Tapp.

use serde::{Deserialize, Serialize};

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
