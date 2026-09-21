# Security Baseline — Desktop Foundation

Checkpoint: 2026-09-20 America/Chicago

## Authority boundary

Harness Desktop is a client of Harness Core contract v1. The renderer is not a coding authority and the Rust host broker is not a replacement for Project/Session, signed workspace leases, capability enforcement, Landlock or Resource Governor policy.

## Renderer and IPC

- Exactly five commands are registered: `runtime_status`, `runtime_provision`, `runtime_start`, `runtime_stop` and `runtime_logs`.
- `tauri_build::AppManifest` generates the command ACL.
- Main-window capability is local-only and Windows-only.
- The capability grants only the five matching `allow-runtime-*` permissions.
- `core:default` is intentionally not granted.
- No shell, process or log plugin is installed.
- The renderer cannot supply an executable or argv. The only renderer-supplied
  value that reaches the broker is `runtime_logs(lines)`, a `u16` clamped to
  `1..=500` and appended as a bare number after a fixed `--lines` flag.
- CSP permits only local application resources and Tauri IPC/asset schemes required by the shell.
- Production DevTools are disabled.

## WSL operations

Every operation runs a fixed argument vector against `wsl.exe`, resolved from an
absolute `%SystemRoot%\System32` path rather than through `PATH`, so the broker
does not depend on ambient path resolution. The process is spawned with
`CREATE_NO_WINDOW`, stdin closed, and stdout/stderr captured on separate reader
threads bounded to `MAX_CAPTURE_BYTES`. Each call is bounded by
`COMMAND_TIMEOUT_SECS`; on expiry the child is killed and the outcome is
reported as `timed_out`, never as success.

- `runtime_status` — `wsl.exe --status`; classified ready, stopped, missing or unavailable.
- `runtime_start` / `runtime_stop` — fixed `systemctl start|stop rhodiz-harness-bootstrap.service`
  inside the managed distro. Both are serialized; a concurrent attempt returns
  `blocked` rather than racing. See "Lifecycle exclusion" below.
- `runtime_logs` — fixed `journalctl --unit … --lines <clamped>`. Output is line-
  and length-bounded, and any line matching a sensitive phrase or a credential
  token prefix is replaced by `[REDACTED SENSITIVE LOG LINE]`. Prefix markers are
  matched only at token boundaries, so ordinary lines such as `disk-usage` are
  not redacted; JWT-shaped values are caught by their `eyJ` prefix even without
  a labelling phrase. stderr is captured but never reaches the renderer: it is
  used only to bound the payload, which keeps host paths and command errors out
  of the UI at the cost of some diagnostic detail.
- `runtime_provision` — **fails closed**. It performs no work and returns
  `blocked`, because signed runtime manifest verification does not exist yet.
  Provisioning by arbitrary URL is deliberately not implemented. It takes the
  lifecycle lock anyway, so the call site is already correct for the day it does
  mutate.

Every command is declared `#[tauri::command(async)]` so it runs on Tauri's sync
threadpool; a non-async command body would block the UI thread for up to
`COMMAND_TIMEOUT_SECS`.

## Lifecycle exclusion

Mutating operations hold two guards:

1. A process-wide `Mutex`, which rejects a second concurrent call inside this
   process.
2. A lock file under `%LOCALAPPDATA%`, opened denying all sharing.

The second guard exists because the first is not sufficient on its own: this
build does not enforce single-instance, so a second application process would
otherwise drive the same systemd unit concurrently with its own private mutex.
Windows releases the file handle when a process dies, so a crash cannot strand
the lock. A lock file that cannot be created is treated as contention, so the
operation fails closed rather than proceeding unguarded.

**Uncertified:** the cross-process behaviour has been type-checked but not
exercised on Windows. Two real processes contending for this lock is untested
until it runs on a Windows host.

Docker, Core, Route, Memory and Providers remain unavailable until their typed
probes exist.

## Dependency evidence

JavaScript dependency audit reports **0 known vulnerabilities** for both the full and production dependency sets.

RustSec reports **0 known vulnerabilities** but currently reports seven warnings in the global lockfile:

- `proc-macro-error 1.0.4` — unmaintained; absent from the Windows target graph.
- `glib 0.18.5` — unsound advisory; absent from the Windows target graph.
- five `unic-* 0.9.0` maintenance advisories — present transitively on Windows through `urlpattern 0.3 -> tauri-utils 2.9.3`.

The five Windows warnings are upstream maintenance warnings, not known vulnerability advisories. Tauri 2.11.5 currently depends through `tauri-utils 2.9.3`, which requires the `urlpattern 0.3` line. They remain tracked debt and must be reevaluated on each Tauri update; they are not described as resolved.

## Local certification evidence

- Oxlint: 0 warnings / 0 errors.
- Vitest: 15/15 PASS.
- Executable TypeScript/React coverage: 100% statements, branches, functions and lines.
- Production renderer build: PASS.
- Rust broker-core: 9/9 PASS.
- `cargo fmt --check`: PASS.
- Clippy with `-D warnings`: PASS.
- `npm run verify:portable`: PASS end to end.
- npm audit, full and production sets: 0 vulnerabilities.
- Test repeat: Vitest 5/5 rounds; Rust broker-core 5/5 rounds.
- `git diff --check` against the base branch: PASS.

### Coverage of the native broker on this checkpoint

`verify:portable` does not compile `src-tauri/src/broker.rs`: `clippy:rust` is
scoped to `broker-core`, and the Tauri app crate does not build on a plain Linux
host. The full `cargo check --target x86_64-pc-windows-gnu` cross-check could not
run in this environment either, because `tauri-winres` requires the mingw
`x86_64-w64-mingw32-windres` binary, which is not installed.

`verify:windows` now runs `clippy:tauri`, which compiles the whole workspace
with `-D warnings`. Until this checkpoint the only gate touching `broker.rs` was
a warning-tolerant `cargo check`, which is how a lifecycle lock that was never
called, and a dead import, both passed CI.

Instead, the broker's Windows code path was type-checked for
`x86_64-pc-windows-gnu` in an isolated crate that depends on the real
`broker-core` and carries the module verbatim minus its `#[tauri::command]`
attributes: PASS with zero warnings. That covers `std::os::windows::process::CommandExt`,
`creation_flags`, `wait_timeout::ChildExt` and the lifecycle-lock wiring. The
`#[tauri::command(async)]` attribute itself was confirmed against
`tauri-macros 2.6.3`, which lists `async` as an accepted attribute and maps a
non-async function body to the `sync_threadpool` execution context.

This is narrower than a native build and is **not** Windows certification. The
Windows CI job remains the first place `broker.rs` is compiled in full.

## Evidence limitation

The relay can exercise the pure Rust broker and cross-check the Windows Tauri code path, but it is not a Windows host. Native WebView2 behavior, WSL2 installation/provisioning, MSI/NSIS packaging, reboot recovery, Windows firewall behavior and signed updater flows remain uncertified until executed on an approved Windows test environment.

A Linux cross-check must never be reported as native Windows certification.
