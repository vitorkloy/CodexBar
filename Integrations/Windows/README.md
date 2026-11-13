# CodexBar for Windows

A Qt 6 tray shell over the shared CodexBar CLI. Same desktop as Linux, with Windows
credential roots (`%APPDATA%`, `%USERPROFILE%`) and a Startup-folder login item.

The Swift `codexbar.exe` CLI owns provider fetching. Keep `CodexBar_CodexBarCore.bundle`
beside the CLI. See [docs/windows.md](../../docs/windows.md) for provider coverage and gaps.

## Install a CI zip

1. Download the `desktop-windows-x64` artifact from the Windows workflow.
2. Unzip it.
3. From PowerShell in that folder: `.\install.ps1`
4. Start `%LOCALAPPDATA%\Programs\CodexBar\codexbar-windows.exe`

`install.ps1` copies the CLI, resource bundle, and tray app into
`%LOCALAPPDATA%\Programs\CodexBar` and can add a Startup shortcut.

## Build from source

See [docs/windows.md](../../docs/windows.md). The Qt project file is still
`Integrations/Linux/codexbar-linux.pro`; on Windows qmake names the binary
`codexbar-windows`.
