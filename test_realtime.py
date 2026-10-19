#!/usr/bin/env python3
"""Screenshot-streaming integration test for tapp realtime server."""

import asyncio
import json
import sys
from datetime import datetime

WS_URL = "ws://localhost:8765"


async def test_screenshot_streaming():
    """Test screenshot streaming through websocket."""
    import websockets

    results = {
        "timestamp": datetime.now().isoformat(),
        "steps": [],
        "success": False,
        "error": None,
    }

    try:
        async with websockets.connect(WS_URL, close_timeout=5, open_timeout=5) as ws:
            await ws.send(json.dumps({"type": "ping"}))
            response = await asyncio.wait_for(ws.recv(), timeout=5)
            ping_data = json.loads(response)
            results["steps"].append({
                "step": "ping",
                "status": "pass" if ping_data.get("type") == "pong" else "fail",
                "data": ping_data,
            })

            await ws.send(json.dumps({"type": "screenshot"}))
            response = await asyncio.wait_for(ws.recv(), timeout=30)
            screenshot_data = json.loads(response)

            b64_len = len(screenshot_data.get("data", {}).get("base64", ""))
            width = screenshot_data.get("data", {}).get("width", 0)
            height = screenshot_data.get("data", {}).get("height", 0)

            results["steps"].append({
                "step": "screenshot",
                "status": "pass" if b64_len > 0 else "fail",
                "b64_length": b64_len,
                "width": width,
                "height": height,
            })

            await ws.send(json.dumps({"type": "screenshot"}))
            response2 = await asyncio.wait_for(ws.recv(), timeout=30)
            screenshot_data2 = json.loads(response2)

            b64_len2 = len(screenshot_data2.get("data", {}).get("base64", ""))
            results["steps"].append({
                "step": "screenshot_repeat",
                "status": "pass" if b64_len2 > 0 else "fail",
                "b64_length": b64_len2,
            })

            results["success"] = (
                results["steps"][0]["status"] == "pass"
                and results["steps"][1]["status"] == "pass"
                and results["steps"][2]["status"] == "pass"
            )

    except Exception as e:
        results["error"] = f"{type(e).__name__}: {e}"

    return results


if __name__ == "__main__":
    result = asyncio.run(test_screenshot_streaming())
    print(json.dumps(result, indent=2))
    sys.exit(0 if result["success"] else 1)
