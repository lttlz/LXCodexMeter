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
  "scripts\security-audit.ps1"
)

$files = Get-ChildItem -Path $root -Recurse -File | Where-Object {
  $_.FullName -notmatch "node_modules" -and
  $_.FullName -notmatch "target" -and
  $_.FullName -notmatch "dist" -and
  $_.Extension -notin @(".png", ".ico", ".zip")
}

$hits = @()
foreach ($file in $files) {
  $rel = Resolve-Path -Path $file.FullName -Relative
  $rel = $rel.TrimStart(".", "\", "/")
  if ($allowFiles -contains $rel) { continue }

  $text = Get-Content -LiteralPath $file.FullName -Raw -ErrorAction SilentlyContinue
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
