# Release Guide

This document describes how releases and automatic updates are configured for LX Codex Meter.

## GitHub Actions release workflow

`.github/workflows/release.yml` builds Windows x64 installers automatically when a tag matching `v*` is pushed.

The workflow:

1. Builds the Tauri app for `x86_64-pc-windows-msvc`.
2. Produces the NSIS setup exe, the MSI installer, and the updater signature (`.sig`).
3. Generates the `latest.json` updater manifest and uploads it together with all assets to a GitHub Release.
4. Applies the bilingual release notes from `RELEASE_BODY.md` and publishes the release.

## Required GitHub Secrets

Configure the following repository secrets before pushing a release tag:

| Secret | Description |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | The Tauri updater signing private key (contents of the `.key` file). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The private key password. Leave empty if the key was generated without a password. |

The `GITHUB_TOKEN` is provided automatically by Actions and does not need to be configured.

> The private key must never be committed to the repository. It is stored only in GitHub Secrets and on the local machine that generated it.

## Public key

The updater public key is committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`. This is safe to share — it is only used to verify update signatures.

## Updater endpoints

The Tauri updater checks the following endpoints in order:

1. `https://github.com/lttlz/LXCodexMeter/releases/latest/download/latest.json` — generated automatically by the release workflow.
2. `https://gitee.com/lttlz/LXCodexMeter/raw/main/update/latest.json` — mainland China mirror.

### Gitee manifest sync

After a GitHub release is published, the `update/latest.json` file in this repository should be updated with the new version, signature, and URL, then pushed to Gitee so the mainland China endpoint stays current. The signature value comes from the `.sig` file uploaded to the GitHub release.

## Local build

```bash
npm ci
npm run tauri:build
```

Signed updater artifacts are produced when `TAURI_SIGNING_PRIVATE_KEY` is present in the environment.
