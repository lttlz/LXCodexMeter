# LX Codex Meter — Agent Instructions

## Project goal

Build a Windows local-only tray and floating-window Codex quota meter.

## Required security boundary

Do not implement any feature that:

- reads browser cookies
- imports browser cookies
- asks for OpenAI API keys
- reads `~/.codex/auth.json`
- copies, logs, saves, or displays access tokens or refresh tokens
- reads Codex conversation/session contents
- scans project directories
- scans user disks broadly
- adds telemetry, analytics, or crash reporting SDKs
- adds hidden auto-update SDKs that silently phone home or collect user data

The official Tauri v2 updater (`tauri-plugin-updater`) is explicitly allowed: it is open-source, signature-verified, contacts only the configured GitHub/Gitee release endpoints, collects no user data, and is triggered by the app at startup or by explicit user action. This exception does not permit any other telemetry, analytics, or background data collection.
- opens a network listener other than a localhost-only Codex app-server transport explicitly requested by the user

The intended default data source is:

```text
codex.exe app-server --stdio
```

Allowed JSON-RPC account methods:

```text
initialize
initialized
account/read
account/rateLimits/read
account/usage/read
```

Do not add write/account mutation methods such as logout, login, consume reset credits, or send email nudges unless the user explicitly asks and a separate security review is done.

## Codex executable discovery

Prefer user-writable installed copies:

```text
%LOCALAPPDATA%\OpenAI\Codex\bin\*\codex.exe
%USERPROFILE%\.codex\plugins\.plugin-appserver\codex.exe
```

Do not run or modify files inside:

```text
C:\Program Files\WindowsApps
```

Do not change WindowsApps permissions.

## Logging

Default behavior must not log:

- tokens
- emails
- prompts
- absolute project paths
- Codex conversation content

Error text may include a redacted executable path for diagnostics.

## Build

```powershell
npm install
npm run build
npm run tauri:dev
npm run tauri:build
```

Run the audit script before packaging:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\security-audit.ps1
```
