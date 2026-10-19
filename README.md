# Tapp

Computer use for Pantheon. Desktop automation via cua-driver, cursor overlay for trust.

## What it is

Tapp is a separate repo that integrates with Pantheon via MCP. It gives the agent full desktop automation: screenshots, clicks, typing, scrolling, dragging. The cursor overlay shows what the agent is about to click before it clicks.

## Why separate

Not every Pantheon user wants computer use. Tapp is opt-in. Add it to your Pantheon config as an MCP server and it fits like a glove.

## Architecture

Tapp follows an adapter pattern: one core engine, multiple harness integrations.

- `tapp-core`: shared types and cua client. talks to cua-driver over a unix socket.
- `tapp-mcp`: MCP server exposing `tapp_*` tools to Pantheon or any MCP harness.
- `tapp-overlay`: cursor overlay binary. shows where the agent is about to click.
- `realtime_server.py`: websocket server for streaming screenshots and executing actions in realtime.

All adapters share `tapp-core`. each adapter handles harness specific concerns (stdio vs websocket, tool discovery vs streaming, auth, rate limiting) while the core stays the same.

### Input translation

cua-driver handles low level input on Linux. window targeted input (naming a specific window) is translated to desktop level XTEST calls when needed, because GTK4 and Qt ignore `XSendEvent`. Tapp does not implement this translation itself. it delegates to cua-driver, which already handles the X11 input layer.

### Diagnostic logging

Both `tapp-mcp` and `realtime_server.py` log events to `~/.cache/tapp/events.jsonl`. the log records event types and metadata only: no tool arguments, no results, no content. set `TAPP_DIAGNOSTIC_LOG=0` to disable.

## Tools

- `tapp_screenshot`: take a screenshot (includes coordinate frame)
- `tapp_click`: click at a point
- `tapp_click_element`: click an element by token from tapp_window_state
- `tapp_type`: type text
- `tapp_key`: press a key
- `tapp_scroll`: scroll at a point
- `tapp_drag`: drag from one point to another
- `tapp_list_windows`: list all open windows
- `tapp_bring_to_front`: bring a window to front
- `tapp_window_state`: get accessibility tree for a window
- `tapp_verify`: verify state after an action
- `tapp_zoom`: zoom into a window region
- `tapp_invoke_menu`: invoke a menu path in a window
- `tapp_set_window_frame`: set window position and size
- `tapp_launch_app`: launch an application
- `tapp_kill_app`: kill an application by PID
- `tapp_clipboard_read`: read the clipboard
- `tapp_clipboard_write`: write to the clipboard
- `tapp_move_cursor`: move the agent cursor
- `tapp_get_cursor_position`: get cursor position
- `tapp_get_screen_size`: get screen size

## Configuration

The cua-driver binary is resolved from `TAPP_CUA_DRIVER`, falling back to
`cua-driver` on `PATH`.

The realtime websocket server reads its config from the environment:

- `TAPP_MCP_BIN`: path to the `tapp-mcp` binary (or put it on `PATH`)
- `DISPLAY`: X11 display, defaults to `:1`
- `TAPP_WS_BIND`: bind address, defaults to `127.0.0.1`
- `TAPP_WS_PORT`: port, defaults to `8765`
- `TAPP_WS_TOKEN`: when set, clients must authenticate with
  `{"type": "auth", "token": "..."}` as their first message
- `TAPP_OVERLAY_WAIT_MS`: overlay wait before acting, defaults to 100ms

Key modifiers are case-sensitive: `ctrl`, `alt`, `shift`, `meta`. Unknown
modifiers are rejected.

## Install

```bash
cargo build --release
```

## Configure

Add to your Pantheon config:

```toml
[mcp_servers.tapp]
command = "tapp-mcp"
args = []
```

## License

MIT
