#!/usr/bin/env python3
"""Realtime computer use stream server for tapp. Uses tapp-mcp for permissions."""

import asyncio
import hmac
import json
import os
import shutil
import websockets

# tapp-mcp binary: explicit path via TAPP_MCP_BIN, otherwise whatever is on PATH.
TAPP_MCP = os.environ.get("TAPP_MCP_BIN") or shutil.which("tapp-mcp")
if not TAPP_MCP:
    raise SystemExit(
        "tapp-mcp not found: set TAPP_MCP_BIN or put tapp-mcp on PATH"
    )
DISPLAY = os.environ.get("DISPLAY", ":1")

# Network surface: localhost-only by default, override with TAPP_WS_BIND.
# When TAPP_WS_TOKEN is set, every client must authenticate with its first
# message: {"type": "auth", "token": "<token>"}.
WS_BIND = os.environ.get("TAPP_WS_BIND", "127.0.0.1")
WS_PORT = int(os.environ.get("TAPP_WS_PORT", "8765"))
WS_TOKEN = os.environ.get("TAPP_WS_TOKEN")


class TappMCP:
    """JSON-RPC client for tapp-mcp."""

    def __init__(self):
        self.proc = None
        self.req_id = 0

    async def start(self):
        env = os.environ.copy()
        env["DISPLAY"] = DISPLAY
        self.proc = await asyncio.create_subprocess_exec(
            TAPP_MCP,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
            limit=1024 * 1024,
        )
        await self._request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "tapp-realtime", "version": "0.1.0"},
        })
        notif = {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
        body = json.dumps(notif)
        self.proc.stdin.write(body.encode() + b"\n")
        await self.proc.stdin.drain()

    async def _request(self, method, params, timeout=15):
        self.req_id += 1
        req = {"jsonrpc": "2.0", "id": self.req_id, "method": method, "params": params}
        body = json.dumps(req)
        self.proc.stdin.write(body.encode() + b"\n")
        await self.proc.stdin.drain()

        async def read_response():
            line = await self.proc.stdout.readuntil(b'\n')
            if not line:
                raise Exception("tapp-mcp stdout closed")
            return json.loads(line.decode())

        try:
            return await asyncio.wait_for(read_response(), timeout=timeout)
        except asyncio.TimeoutError:
            raise Exception(f"timeout waiting for response to {method}")

    async def call_tool(self, name, args):
        result = await self._request("tools/call", {"name": name, "arguments": args})
        if "error" in result:
            raise Exception(result["error"])
        return result.get("result", {})

    async def screenshot(self):
        result = await self.call_tool("tapp_screenshot", {})
        content = result.get("content", [])
        for item in content:
            if item.get("type") == "text":
                text = item.get("text", "{}")
                data = json.loads(text)
                b64 = data.get("base64", "")
                if b64:
                    return {
                        "base64": b64,
                        "width": data.get("width", 1920),
                        "height": data.get("height", 1080),
                    }
        return None

    async def execute_action(self, action):
        tool = action.get("tool")
        args = action.get("args", {})
        try:
            await self.call_tool(f"tapp_{tool}", args)
            return True
        except Exception as e:
            print(f"action failed: {e}")
            return False


async def authenticate(websocket) -> bool:
    """First-message token auth. No token configured means no auth required."""
    if not WS_TOKEN:
        return True
    try:
        raw = await asyncio.wait_for(websocket.recv(), timeout=10)
        auth = json.loads(raw)
    except Exception:
        return False
    token = auth.get("token", "") if isinstance(auth, dict) else ""
    return auth.get("type") == "auth" and hmac.compare_digest(token, WS_TOKEN)


class RateLimiter:
    """Simple token bucket rate limiter."""

    def __init__(self, rate: float = 10.0, burst: int = 20):
        self.rate = rate
        self.burst = burst
        self.tokens = float(burst)
        self.last = asyncio.get_event_loop().time()

    async def acquire(self):
        now = asyncio.get_event_loop().time()
        elapsed = now - self.last
        self.last = now
        self.tokens = min(self.burst, self.tokens + elapsed * self.rate)
        if self.tokens < 1:
            wait = (1 - self.tokens) / self.rate
            await asyncio.sleep(wait)
            self.tokens = 0
        else:
            self.tokens -= 1


async def handler(websocket, tapp):
    """Handle a client connection."""
    if not await authenticate(websocket):
        await websocket.close(code=4401, reason="unauthorized")
        return
    limiter = RateLimiter(rate=10.0, burst=20)
    try:
        async for message in websocket:
            await limiter.acquire()
            data = json.loads(message)

            if data.get("type") == "screenshot":
                screenshot = await tapp.screenshot()
                if screenshot:
                    await websocket.send(json.dumps({
                        "type": "screenshot",
                        "data": screenshot
                    }))

            elif data.get("type") == "action":
                success = await tapp.execute_action(data.get("action", {}))
                await websocket.send(json.dumps({
                    "type": "action_result",
                    "success": success
                }))

            elif data.get("type") == "ping":
                await websocket.send(json.dumps({"type": "pong"}))

    except websockets.exceptions.ConnectionClosed:
        pass


async def main():
    """Start the server."""
    tapp = TappMCP()
    await tapp.start()

    async def wrapped_handler(websocket):
        await handler(websocket, tapp)

    if WS_BIND != "127.0.0.1" and not WS_TOKEN:
        print(
            f"WARNING: binding {WS_BIND} without TAPP_WS_TOKEN set. "
            "Anyone on the network can drive the desktop. Set TAPP_WS_TOKEN."
        )
    print(f"tapp realtime server on {WS_BIND}:{WS_PORT}")
    server = await websockets.serve(wrapped_handler, WS_BIND, WS_PORT)
    await server.wait_closed()


if __name__ == "__main__":
    asyncio.run(main())
