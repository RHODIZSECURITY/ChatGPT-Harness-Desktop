# Security Baseline — Desktop Foundation

Checkpoint: 2026-09-20 America/Chicago

## Authority boundary

Harness Desktop is a client of Harness Core contract v1. The renderer is not a coding authority and the Rust host broker is not a replacement for Project/Session, signed workspace leases, capability enforcement, Landlock or Resource Governor policy.

## Renderer and IPC

- Exactly seven commands are registered: `runtime_status`, `runtime_provision`,
  `runtime_start`, `runtime_stop`, `runtime_verify`, `runtime_repair` and
  `runtime_logs`.
- `tauri_build::AppManifest` generates the command ACL.
- Main-window capability is local-only and Windows-only.
- The capability grants only the seven matching `allow-runtime-*` permissions.
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
- `runtime_verify` — **read-only diagnosis**. One fixed probe,
  `systemctl is-active rhodiz-harness-bootstrap.service`, classified by exit
  code: `active`; `inactive` (distribution answered, unit is not running);
  `distro_unreachable`; `wsl_missing`; or `unknown` (timeout). It deliberately
  takes **no lifecycle lock**: holding one would answer `blocked` during a
  start or a stop, exactly when a diagnosis is most needed. That is safe
  because `systemctl is-active` mutates nothing. The price of being lock-free
  is that the answer is an instantaneous snapshot: a probe issued while a
  start, stop or repair is mid-flight can report the transient value it
  happens to observe; callers needing a settled state re-verify after the
  mutation returns. It deliberately probes
  **only** WSL, distro reachability and the bootstrap unit — Docker, Core,
  Route, Memory and Providers are reported as unprobed so the renderer cannot
  mistake "not probed" for "verified healthy". It parses no stdout; `wsl.exe`
  emits UTF-16LE and classification is by exit code, the pattern every command
  but the provisioning version probe below uses.
- `runtime_repair` — **explicit, bounded mutation**, never automatic. Under the
  lifecycle lock it runs exactly two fixed commands against the unit:
  `systemctl reset-failed` (best-effort cleanup of a latched start-limit
  failure, its outcome deliberately ignored) and `systemctl restart`, whose
  outcome decides the result. When the distribution is unreachable, `restart`
  fails the same way, so the resulting diagnosis stays correct. It never
  provisions, installs or deletes anything. A **distro-level restart is
  deliberately omitted**: that decision belongs to the operator because of its
  blast radius and precedent, and the underlying failure is not yet
  reproducible until provisioning and Core land; revisit once tasks 4.2/4.3
  exist.
  **Assumption, not measured evidence:** `systemctl is-active` exiting `3` is
  treated as "distribution answered, unit inactive/failed/unknown", and any
  *other* non-zero exit as "distribution unreachable". This mapping has not
  been exercised against a real `wsl.exe`; confirming it belongs to Windows
  certification, and the broker-core constant documents it as an assumption.
- `runtime_provision` — **fails closed on every branch**. It still creates,
  downloads and installs nothing, because signed runtime manifest verification
  does not exist yet, and provisioning by arbitrary URL is deliberately not
  implemented. What it now does is *diagnose* before refusing, under the
  lifecycle lock, so the call site is already correct for the day it mutates.
  It runs the fixed status probe and, **only if that probe proves `wsl.exe`
  answers at all**, a second fixed `wsl.exe --version`. Gating the second spawn
  behind the first keeps an absent or wedged WSL to one `COMMAND_TIMEOUT_SECS`
  rather than two, and the preflight ignores the version on those branches
  anyway. The four states are distinguished **in the `detail` string only** — every branch returns `OperationState::Blocked` because the signed manifest gate is the ultimate barrier. The states are: WSL absent, WSL unprobeable, WSL present but older than the pinned minimum, and WSL sufficient — where the only remaining obstacle is the manifest gate, so that branch defers to the same refusal message rather than restating it.

  **The one place the broker parses stdout for anything but logs.**
  `wsl.exe --version` writes UTF-16LE, so the version cannot be read from an
  exit code. The exception is deliberately narrow and fails closed at every
  step: a non-success capture is not decoded, a **truncated** capture is
  discarded rather than parsed (the cut can land mid-version and yield a
  plausible but wrong triple that would wrongly clear the minimum), a decode
  that finds no dotted triple yields no version, and a version that cannot be
  established is a **refusal to provision**, never an assumed-sufficient pass.
  `tests/security-contract.test.ts` pins the argv, the truncation guard and the
  refusal message against drift.

Every command is declared `#[tauri::command(async)]` so it runs on Tauri's sync
threadpool; a non-async command body would block the UI thread for up to
`COMMAND_TIMEOUT_SECS`.

## Lifecycle exclusion

Mutating operations hold two guards:

1. A process-wide `Mutex`, which rejects a second concurrent call inside this
   process. `runtime_verify` holds neither guard, by design: it issues no
   mutation.
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
- Vitest: 17/17 PASS.
- Executable TypeScript/React coverage: 100% statements, branches, functions and lines.
- Production renderer build: PASS.
- Rust broker-core: 26/26 PASS.
- `cargo fmt --check`: PASS.
- Clippy with `-D warnings`: PASS.
- `npm run verify:portable`: PASS end to end.
- npm audit, full and production sets: 0 vulnerabilities.
- Test repeat: Vitest 5/5 rounds; Rust broker-core 5/5 rounds.
- Windows-target cross-check: `cargo check --target x86_64-pc-windows-msvc` and
  `cargo clippy --target x86_64-pc-windows-msvc -- -D warnings`: PASS with
  0 errors and 0 warnings over the whole workspace.
- `git diff --check` against the base branch: PASS.

### Coverage of the native broker on this checkpoint

`verify:portable` does not compile `src-tauri/src/broker.rs`: `clippy:rust` is
scoped to `broker-core`, and the Tauri app crate does not link GTK on a plain
Linux host. `clippy:tauri` against the **host** target is red on Linux even at
the base commit: every Windows-only code path is `#[cfg]`-ed out there, so
`-D warnings` flags its imports and helpers as dead. That is a property of the
gate as defined, not of this change, and it means `clippy:tauri` can only be
certified on a Windows host — which the CI job recorded below now does.

What this checkpoint does run, and passes with zero errors and zero warnings:

- `cargo check --target x86_64-pc-windows-msvc` — the real Windows target,
  including `broker.rs` and the Tauri ACL build script, which validates the
  capability manifest against the actual permission set. This required
  `llvm-rc` (from a user-space LLVM 20 installation) for the Windows resource
  step; it compiles every `#[cfg(windows)]` path.
- `cargo clippy --target x86_64-pc-windows-msvc -- -D warnings` over the whole
  workspace. This caught one real finding on the lifecycle lock file
  (`create` without explicit truncate behaviour, fixed by `.truncate(false)`)
  that no warning-tolerant check had surfaced before.
- `cargo clippy -p rhodiz-harness-broker-core --all-targets -- -D warnings` on
  the host, covering all pure logic including the new verify classification.

### Windows CI result on this commit

The `Windows Tauri check` job ran `npm run verify:windows` on a native
`windows-latest` runner for this exact commit (workflow run 35553702789, event
`pull_request`) and **passed**. The script is a `&&` chain, so `verify:portable`,
`check:tauri` and `clippy:tauri` all succeeded there; `clippy:tauri` with
`-D warnings` reported zero errors and zero warnings over both
`rhodiz-harness-desktop` and `rhodiz-harness-broker-core`. `broker.rs` is
therefore compiled and lint-clean against a real MSVC toolchain, which the
Linux cross-check above could only approximate.

That green result is **not** runtime certification and must not be read as one.
`cargo check` and `cargo clippy` do not link a binary and execute no code, and
the CI runner has no WSL2 installation, no `RHODIZ-Harness` distribution and no
bootstrap unit. What stays unverified is therefore unchanged by CI passing:
real `systemctl is-active` exit codes through `wsl.exe` (the exit-3 mapping
remains an assumption), the **actual byte shape of `wsl.exe --version` output**
(the UTF-16LE decoder and the version scan are exercised only against
synthesised bytes, so a real banner that carries no dotted triple would surface
here as a refusal to provision, not as a crash or a wrong pass), `repair`
against a genuinely failed unit, lock contention between two real processes,
and native WebView2 behaviour. Those require an approved Windows test
environment with WSL2, which a CI type-check is not.

## Evidence limitation

The relay can exercise the pure Rust broker and cross-check the Windows Tauri code path, but it is not a Windows host. Native WebView2 behavior, WSL2 installation/provisioning, MSI/NSIS packaging, reboot recovery, Windows firewall behavior and signed updater flows remain uncertified until executed on an approved Windows test environment.

A Linux cross-check must never be reported as native Windows certification.
