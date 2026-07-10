$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$forbidden = @(
  "cookie",
  "cookies",
  "auth.json",
  "access_token",
  "refresh_token",
  "authorization:",
  "bearer ",
  "localstorage",
  "sessionstorage",
  "sqlite",
  "appdata\\local\\google\\chrome",
  "appdata\\roaming\\mozilla",
  "telemetry",
  "analytics",
  "sentry",
  "posthog",
  "segment.com"
)

$allowFiles = @(
  "README.md",
  "AGENTS.md",
  "RELEASE_BODY.md",
  "scripts\security-audit.ps1"
)

$excludePrefixes = @(
  ".tmp-",
  ".workbuddy/memory/",
  "workbuddy/memory/",
  "node_modules/",
  "dist/",
  "src-tauri/target/",
  "src-tauri/gen/"
)

$excludeFiles = @(
  "package-lock.json",
  "src-tauri/Cargo.lock"
)

$files = git -C $root ls-files | Where-Object {
  $rel = $_ -replace "\\", "/"
  if ($excludeFiles -contains $rel) { return $false }
  foreach ($prefix in $excludePrefixes) {
    if ($rel.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
      return $false
    }
  }
  return $true
}

$hits = @()
foreach ($rel in $files) {
  $rel = $rel -replace "/", "\"
  if ($allowFiles -contains $rel) { continue }

  $path = Join-Path $root $rel
  $text = Get-Content -LiteralPath $path -Raw -ErrorAction SilentlyContinue
  if ($null -eq $text) { continue }
  foreach ($term in $forbidden) {
    if ($text.ToLowerInvariant().Contains($term.ToLowerInvariant())) {
      $hits += [PSCustomObject]@{ File = $rel; Term = $term }
    }
  }
}

if ($hits.Count -gt 0) {
  Write-Host "[FAIL] Found forbidden security-sensitive terms:" -ForegroundColor Red
  $hits | Format-Table -AutoSize
  exit 1
}

Write-Host "[OK] Security audit passed. No forbidden terms found outside allowlisted documentation files." -ForegroundColor Green
