# Installs a packaged CodexBar Windows zip into %LOCALAPPDATA%\Programs\CodexBar.
param(
    [switch]$NoStartup
)

$ErrorActionPreference = "Stop"
$Source = Split-Path -Parent $MyInvocation.MyCommand.Path
$Destination = Join-Path $env:LOCALAPPDATA "Programs\CodexBar"
New-Item -ItemType Directory -Force -Path $Destination | Out-Null

Get-ChildItem $Source -Force | Where-Object {
    $_.Name -notin @("install.ps1")
} | ForEach-Object {
    Copy-Item -Recurse -Force $_.FullName (Join-Path $Destination $_.Name)
}

$Tray = Join-Path $Destination "codexbar-windows.exe"
if (-not $NoStartup) {
    $Startup = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"
    New-Item -ItemType Directory -Force -Path $Startup | Out-Null
    $Bat = Join-Path $Startup "CodexBar.bat"
    Set-Content -Path $Bat -Value "@echo off`r`nstart `"`" `"$Tray`" --background`r`n" -Encoding ASCII
}

Write-Host "Installed CodexBar to $Destination"
Write-Host "CLI: $(Join-Path $Destination 'codexbar.exe')"
Write-Host "Tray: $Tray"
