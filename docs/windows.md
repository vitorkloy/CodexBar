---
summary: "Windows tray preview: Swift CLI plus the Qt desktop, local credential paths, and known gaps."
read_when:
  - Building or testing the Windows tray
  - Changing Windows credential roots or the windows.yml workflow
---

# Windows tray

This fork ships a **preview** Windows tray that reuses the Linux Qt desktop and the shared Swift `codexbar` CLI. It is not a macOS feature parity port. Provider fetching stays in the CLI; the desktop owns polling, settings, and the tray icon.

## What works

- Swift CLI (`CodexBarCLI`, packaged as `codexbar.exe`) on Windows, with macOS-only UI, Keychain, Sparkle, and widgets gated behind `#if os(macOS)`.
- Qt 6 tray + Usage & Spend / Settings windows, talking to the CLI over a local named pipe (same IPC as Linux).
- v1 local credentials for **Codex**, **Claude**, **Cursor**, **Gemini**, and **Copilot**:
  - Codex: `%USERPROFILE%\.codex\auth.json` (`CODEX_HOME` still wins)
  - Claude: `%USERPROFILE%\.claude\` and `.credentials.json` (`CLAUDE_CONFIG_DIR` still wins)
  - Cursor: `%APPDATA%\Cursor\User\globalStorage\state.vscdb` and `%APPDATA%\cursor\auth.json`
  - Gemini: `%USERPROFILE%\.gemini\oauth_creds.json`
  - Copilot: `COPILOT_API_TOKEN` or accounts saved in `%APPDATA%\codexbar\config.json`
- GitHub Actions `windows.yml` on `windows-latest` builds both binaries and uploads a zip artifact. No release, tag, or installer publication.

## What is out of scope / skipped

- Browser cookie import and Keychain-backed sessions (SweetCookieKit / macOS Security). Copilot **budget extras** that need github.com cookies are skipped.
- PTY CLI probes (`openpty`, interactive Claude/Codex TTY sessions). File-backed OAuth and `Foundation.Process` subprocesses remain.
- `codexbar serve` (loopback HTTP). The tray talks to the CLI over a named pipe and by spawning `codexbar.exe`.
- Linux D-Bus notifications. The Windows tray tooltip still updates; desktop toasts are off.
- Homebrew/ChatGPT.app binary discovery, Sparkle updates, widgets, and the native macOS menu bar app.
- A signed MSI/MSIX. CI ships a zip plus `install.ps1`.

## Build

Requires Windows 10+, Visual Studio C++ tools (GitHub `windows-latest` already has them), Swift 6.3.3, Qt 6.4+, and SQLite development files.

```powershell
swift build -c release --product CodexBarCLI
mkdir .local\windows-build
cd .local\windows-build
qmake ..\..\Integrations\Linux\codexbar-linux.pro
nmake
```

Package with `python Integrations/Windows/package.py --version 0.0.0-ci --cli <codexbar.exe> --desktop <codexbar-windows.exe>`.
