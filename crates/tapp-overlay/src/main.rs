//! Tapp cursor overlay.
//!
//! Shows a visual cursor that moves to the target before the agent clicks.
//! This is the Clicky pattern: the user sees what the agent is about to do.

use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tapp_core::types::{OverlayEvent, Point};
use tracing;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Arc as XArc, ChangeGCAux, ConfigureWindowAux, ConnectionExt, CreateGCAux, CreateWindowAux,
    EventMask, Gcontext, Rectangle, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;

/// Cursor overlay state.
struct OverlayState {
    conn: Arc<RustConnection>,
    window: Window,
    gc: Gcontext,
    cursor_x: i32,
    cursor_y: i32,
    visible: bool,
    label: Option<String>,
}

impl OverlayState {
    /// Connect to X11 and create the overlay window.
    fn new(display: &str) -> Result<Self> {
        let (conn, screen_num) = x11rb::connect(Some(display))?;
        let conn = Arc::new(conn);
        let screen = &conn.setup().roots[screen_num];

        // Create a small fixed size window (50x50) that follows the cursor
        let window = conn.generate_id()?;
        conn.create_window(
            0,
            window,
            screen.root,
            0,
            0,
            50,
            50,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .override_redirect(1u32)
                .background_pixel(0x000000)
                .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY),
        )?;

        // Create a graphics context
        let gc = conn.generate_id()?;
        conn.create_gc(
            gc,
            window,
            &CreateGCAux::new()
                .foreground(0x00FF00)
                .background(0x000000)
                .line_width(2)
                .graphics_exposures(0u32),
        )?;

        conn.map_window(window)?;
        conn.flush()?;

        Ok(Self {
            conn,
            window,
            gc,
            cursor_x: 0,
            cursor_y: 0,
            visible: false,
            label: None,
        })
    }

    /// Draw a circle at the cursor position.
    fn draw_cursor(&self) -> Result<()> {
        // Clear the window
        self.conn.poly_fill_rectangle(
            self.window,
            self.gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: 50,
                height: 50,
            }],
        )?;

        // Draw a filled circle
        self.conn.poly_fill_arc(
            self.window,
            self.gc,
            &[XArc {
                x: 5,
                y: 5,
                width: 40,
                height: 40,
                angle1: 0,
                angle2: 360 * 64,
            }],
        )?;

        // Draw a border
        self.conn.poly_arc(
            self.window,
            self.gc,
            &[XArc {
                x: 5,
                y: 5,
                width: 40,
                height: 40,
                angle1: 0,
                angle2: 360 * 64,
            }],
        )?;

        self.conn.flush()?;
        Ok(())
    }

    /// Move the cursor to a point and show it with fade in.
    fn show_cursor(&mut self, point: Point, label: Option<String>) -> Result<()> {
        self.cursor_x = point.x;
        self.cursor_y = point.y;
        self.visible = true;
        self.label = label;

        // Move the window to the cursor position
        self.conn.configure_window(
            self.window,
            &ConfigureWindowAux::new()
                .x(point.x - 25)
                .y(point.y - 25),
        )?;

        // Fade in
        self.fade_in()?;
        Ok(())
    }

    /// Fade in the cursor.
    fn fade_in(&self) -> Result<()> {
        let steps = 10;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let r = (0.0 + (0x00 as f64 - 0x00 as f64) * t) as u32;
            let g = (0.0 + (0xFF as f64 - 0x00 as f64) * t) as u32;
            let b = (0.0 + (0x00 as f64 - 0x00 as f64) * t) as u32;
            let color = (r << 16) | (g << 8) | b;

            self.conn.change_gc(
                self.gc,
                &ChangeGCAux::new().foreground(color),
            )?;

            self.draw_cursor()?;
            self.draw_label()?;
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        Ok(())
    }

    /// Draw the label text below the cursor.
    fn draw_label(&self) -> Result<()> {
        if let Some(ref text) = self.label {
            let text_bytes = text.as_bytes();
            self.conn.poly_text8(
                self.window,
                self.gc,
                5,
                45,
                text_bytes,
            )?;
            self.conn.flush()?;
        }
        Ok(())
    }

    /// Hide the cursor with fade out.
    fn hide_cursor(&mut self) -> Result<()> {
        // Fade out
        self.fade_out()?;
        self.visible = false;
        self.label = None;
        // Move the window off-screen
        self.conn.configure_window(
            self.window,
            &ConfigureWindowAux::new().x(-100).y(-100),
        )?;
        self.conn.flush()?;
        Ok(())
    }

    /// Fade out the cursor.
    fn fade_out(&self) -> Result<()> {
        let steps = 10;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let r = (0x00 as f64 + (0x00 as f64 - 0x00 as f64) * t) as u32;
            let g = (0xFF as f64 + (0x00 as f64 - 0xFF as f64) * t) as u32;
            let b = (0x00 as f64 + (0x00 as f64 - 0x00 as f64) * t) as u32;
            let color = (r << 16) | (g << 8) | b;

            self.conn.change_gc(
                self.gc,
                &ChangeGCAux::new().foreground(color),
            )?;

            self.draw_cursor()?;
            self.draw_label()?;
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        Ok(())
    }

    /// Animate the cursor from one point to another.
    fn animate_to(&mut self, from: Point, to: Point, duration_ms: u64) -> Result<()> {
        let steps = 20;
        let step_duration = duration_ms / steps;

        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = from.x as f64 + (to.x - from.x) as f64 * t;
            let y = from.y as f64 + (to.y - from.y) as f64 * t;

            self.cursor_x = x as i32;
            self.cursor_y = y as i32;
            self.visible = true;

            self.conn.configure_window(
                self.window,
                &ConfigureWindowAux::new()
                    .x(self.cursor_x - 25)
                    .y(self.cursor_y - 25),
            )?;

            self.draw_cursor()?;
            std::thread::sleep(std::time::Duration::from_millis(step_duration));
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":1".to_string());
    let mut state = OverlayState::new(&display)?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    writeln!(stdout, "Tapp overlay ready")?;
    stdout.flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: OverlayEvent = serde_json::from_str(&line)?;

        match event {
            OverlayEvent::ShowCursor { point, label } => {
                tracing::info!(?point, ?label, "Show cursor");
                state.show_cursor(point, label)?;
            }
            OverlayEvent::HideCursor => {
                tracing::info!("Hide cursor");
                state.hide_cursor()?;
            }
            OverlayEvent::AnimateTo { from, to, duration_ms } => {
                tracing::info!(?from, ?to, duration_ms, "Animate cursor");
                state.animate_to(from, to, duration_ms)?;
            }
        }
    }

    Ok(())
}
