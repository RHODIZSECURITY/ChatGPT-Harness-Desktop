# Security Baseline — Desktop Foundation

Checkpoint: 2026-09-20 America/Chicago

## Authority boundary

Harness Desktop is a client of Harness Core contract v1. The renderer is not a coding authority and the Rust host broker is not a replacement for Project/Session, signed workspace leases, capability enforcement, Landlock or Resource Governor policy.

## Renderer and IPC

- `runtime_status` is the only registered Tauri application command.
- `tauri_build::AppManifest` generates the command ACL.
- Main-window capability is local-only and Windows-only.
- The capability grants only `allow-runtime-status`.
- `core:default` is intentionally not granted.
- No shell, process or log plugin is installed.
- The renderer cannot supply an executable or argv.
- CSP permits only local application resources and Tauri IPC/asset schemes required by the shell.
- Production DevTools are disabled.

## WSL probe

The current broker operation is fixed to `wsl.exe --status`. Outcomes are classified as ready, stopped or missing; unknown managed-runtime components remain unavailable until their typed probes exist.

This milestone does not expose provision/start/stop/log operations yet.

## Dependency evidence

JavaScript dependency audit reports **0 known vulnerabilities** for both the full and production dependency sets.

RustSec reports **0 known vulnerabilities** but currently reports seven warnings in the global lockfile:

- `proc-macro-error 1.0.4` — unmaintained; absent from the Windows target graph.
- `glib 0.18.5` — unsound advisory; absent from the Windows target graph.
- five `unic-* 0.9.0` maintenance advisories — present transitively on Windows through `urlpattern 0.3 -> tauri-utils 2.9.3`.

The five Windows warnings are upstream maintenance warnings, not known vulnerability advisories. Tauri 2.11.5 currently depends through `tauri-utils 2.9.3`, which requires the `urlpattern 0.3` line. They remain tracked debt and must be reevaluated on each Tauri update; they are not described as resolved.

## Local certification evidence

- Oxlint: 0 warnings / 0 errors.
- Vitest: 9/9 PASS.
- Executable TypeScript/React coverage: 100% statements, branches, functions and lines.
- Production renderer build: PASS.
- Rust broker-core: 3/3 PASS.
- `cargo fmt --check`: PASS.
- Clippy with `-D warnings`: PASS.
- Tauri Windows GNU cross-check: PASS.
- Test repeat: Vitest 10/10 rounds; Rust 10/10 rounds; production build 3/3 rounds.
- `git diff --check`: PASS.

## Evidence limitation

The relay can exercise the pure Rust broker and cross-check the Windows Tauri code path, but it is not a Windows host. Native WebView2 behavior, WSL2 installation/provisioning, MSI/NSIS packaging, reboot recovery, Windows firewall behavior and signed updater flows remain uncertified until executed on an approved Windows test environment.

A Linux cross-check must never be reported as native Windows certification.
