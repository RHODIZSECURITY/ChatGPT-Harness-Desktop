# RHODIZ Harness Desktop Context

Checkpoint: 2026-09-20 America/Chicago
Repository: `/opt/rhodiz-harness-desktop`
Current branch: `feat/windows-desktop-foundation-20260918`
Current milestone: secure Windows Desktop foundation

## Upstream/core state

Harness Core capability/contracts/versioning/realtime front is CLOSED and remains the coding/runtime authority. Desktop consumes Core contract v1, explicit `coding` Sessions, signed workspace leases and authenticated realtime events.

Pinned provenance anchors:
- OpenHands `a0403035a20c91decadd011b907ee5b489f6788b`
- Onyx `e11c874019dbf04032cfc3476d82eba1d069a3d8`
- LibreChat `2452f499a86ae215146d988e9418a481616238ad`

All three pins were verified against the RHODIZSECURITY remotes before this milestone. No upstream code has been imported yet.

## Implemented foundation

- Tauri 2 + React/TypeScript shell.
- Narrow Rust broker and pure `broker-core` crate.
- One typed IPC command: `runtime_status`.
- Fixed Windows probe: `wsl.exe --status`.
- Local-only, Windows-only Tauri capability granting only `allow-runtime-status`.
- Explicit `AppManifest` command ACL; no `core:default`.
- Strict local CSP; production DevTools disabled.
- No generic shell/process/log plugin.

## Local evidence

- frontend tests 9/9 PASS with 100% executable coverage;
- Oxlint 0 warnings / 0 errors;
- renderer production build PASS;
- broker-core Rust tests 3/3 PASS;
- rustfmt PASS and clippy `-D warnings` PASS;
- Windows GNU Tauri cross-check PASS;
- npm audits 0 vulnerabilities;
- RustSec 0 vulnerabilities plus seven tracked warnings documented in `docs/SECURITY_BASELINE.md`;
- Vitest repeat 10/10, Rust repeat 10/10, build repeat 3/3;
- `git diff --check` PASS.

Native Windows MSI/NSIS, WSL2 lifecycle and reboot/network E2E are not yet certified.

## Naming

Visible product and new runtime naming use **Harness** only. The managed distro name is `RHODIZ-Harness`. Existing stable protocol identifiers such as `rhodiz-arnes` are compatibility identifiers and are not renamed in this Desktop milestone.

## next_action

1. Commit the certified repository foundation on `feat/windows-desktop-foundation-20260918`; never publish this milestone directly to `main`.
2. Create the next local feature branch for typed WSL runtime lifecycle.
3. Add fixed/bounded start, stop and logs operations without exposing renderer-controlled executables or argv.
4. Design provisioning around a signed runtime bundle/manifest; do not implement remote bootstrap by arbitrary URL.
5. Add Test-Automator coverage for every operation and failure state before publication/certification.
