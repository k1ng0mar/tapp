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

- `computer_screenshot`: take a screenshot
- `computer_click`: click at a point
- `computer_type`: type text
- `computer_key`: press a key
- `computer_scroll`: scroll at a point
- `computer_drag`: drag from one point to another

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
