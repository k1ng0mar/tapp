# Tapp

Computer use for Pantheon. Desktop automation via cua-driver, cursor overlay for trust.

## What it is

Tapp is a separate repo that integrates with Pantheon via MCP. It gives the agent full desktop automation: screenshots, clicks, typing, scrolling, dragging. The cursor overlay shows what the agent is about to click before it clicks.

## Why separate

Not every Pantheon user wants computer use. Tapp is opt-in. Add it to your Pantheon config as an MCP server and it fits like a glove.

## Architecture

- `tapp-core`: shared types and cua client
- `tapp-mcp`: MCP server exposing computer use tools
- `tapp-overlay`: cursor overlay binary (Clicky pattern)

## Tools

- `tapp_screenshot`: take a screenshot
- `tapp_click`: click at a point
- `tapp_type`: type text
- `tapp_key`: press a key
- `tapp_scroll`: scroll at a point
- `tapp_drag`: drag from one point to another
- `tapp_list_windows`: list all open windows
- `tapp_bring_to_front`: bring a window to front

## Configuration

The cua-driver binary is resolved from `TAPP_CUA_DRIVER`, falling back to
`cua-driver` on `PATH`.

The realtime websocket server reads its config from the environment:

- `TAPP_MCP_BIN`: path to the `tapp-mcp` binary (or put it on `PATH`)
- `TAPP_WS_BIND`: bind address, defaults to `127.0.0.1`
- `TAPP_WS_PORT`: port, defaults to `8765`
- `TAPP_WS_TOKEN`: when set, clients must authenticate with
  `{"type": "auth", "token": "..."}` as their first message

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
