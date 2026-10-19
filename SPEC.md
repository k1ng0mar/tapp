An open-source, cross-platform AI screen companion. The open, portable answer to HeyClicky/Clicky.

## Problem

HeyClicky/Clicky is Mac-only, and Farza's actively-developed version is going closed-source from here. The existing open fork (`farzaa/clicky`) is Swift/AppKit-only; the Windows port (`clicky_windows`) is a separate C#/WPF rewrite with zero shared code. There's no single cross-platform codebase, and no actively maintained community version.

## Target users

- Windows/Linux users who can't use Clicky at all
- Devs who want to self-host with their own API keys instead of paying for a hosted tier
- Contributors who want to extend or fork it

## V1 scope — screen-aware voice, first

- Configurable global push-to-talk hotkey
- Screen capture, multi-monitor aware (active display first)
- Streaming speech-to-text
- Screenshot + transcript + recent history → vision/reasoning model, routed through your model router (swappable backend)
- Point-at-element: coordinate-detection call (Claude Computer Use API, or vision-grounding fallback for non-Claude backends)
- Animated overlay dot/cursor — transparent, always-on-top, cross-platform via Electron
- TTS playback of the reply (swappable provider)
- Short rolling conversation memory (last ~10 turns)
- Self-hosted proxy layer holding API keys — never shipped in the client

## V2 scope — background agents, later

- Background task spawning, delegated to your Nyx/Hermes agent infra
- Task status surfaced in the overlay/panel
- Optional integrations (Notion/Gmail/Calendar-style) as plugins, not core

## Explicitly out of scope for v1

- Native accessibility-tree interaction (AXUIElement/UIA/AT-SPI) — vision-based pointing only
- Multi-user/team features
- Matching HeyClicky's illustrated brand identity — going minimal dev-tool aesthetic (Raycast/Linear-style) instead

## Tech stack

| Piece | Choice |
|---|---|
| Shell | Electron (single codebase, 3 OSes) |
| Screen capture | `desktopCapturer` |
| Hotkey | `globalShortcut` |
| STT | Streaming provider (e.g. AssemblyAI) |
| Reasoning/vision | Your model router (multi-provider) |
| Pointing | Computer-use-style coordinate detection |
| TTS | Swappable (ElevenLabs / local) |
| Key storage | Self-hosted lightweight proxy |
| Agent tasks | Nyx/Hermes |

## v1 success criteria

Push-to-talk → screenshot → transcript → spoken answer + accurate on-screen pointer — working identically on Windows, Linux, and Mac from one codebase, with your own API keys via a self-hosted proxy.
