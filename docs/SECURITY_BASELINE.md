# Security Baseline — Desktop Foundation

Checkpoint: 2026-09-21 America/Chicago

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

Every distro-scoped vector names a **slot** rather than a distro. Two managed
distros can exist at once while an update is staged, and which one is serving is
a fact that lives in the broker's state file, not in the binary. It is read per
command rather than cached at startup, so a swap by a concurrent provision is
picked up by the next command instead of being shadowed by a stale value.

An **unreadable state file is refused, not defaulted**. Before staging existed,
reading a corrupt file as the primary slot could not name the wrong distro
because no other distro existed; now it can, and the wrong guess would start,
restart or read the logs of a runtime the operator is not running. There is no
safe default, so all five lifecycle commands carry the failure out to the
renderer, each in its own result shape — a failed operation, an unhealthy
verify with everything marked unprobed, or an empty log read that is explicitly
not a truncation. Each says why, because a refusal an operator cannot act on is
barely better than a wrong answer.

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
- `runtime_provision` — **the one mutating pipeline, and the only code in the
  broker that opens a network connection.** It no longer merely diagnoses and
  refuses. It runs the fixed status probe and, **only if that probe proves
  `wsl.exe` answers at all**, a second fixed `wsl.exe --version`. Gating the
  second spawn behind the first keeps an absent or wedged WSL to one
  `COMMAND_TIMEOUT_SECS` rather than two. If that preflight blocks, nothing
  further runs and no connection is opened; the four blocking states are
  distinguished **in the `detail` string only**: WSL absent, WSL unprobeable,
  WSL present but older than the pinned minimum, and WSL sufficient.

  Past the preflight it fetches `release.json` and `release.json.sig` from
  `RHODIZ_UPDATE_URL`, verifies the signature, parses the manifest, downloads
  the rootfs, compares a digest, imports the distro, writes `wsl.conf`,
  terminates so systemd boots, installs Docker inside the distro, and only
  then records the release. Failures along that path return
  `OperationState::Failed`, not `Blocked`.

  **What still stops it in a shipped build, and what does not.** No release
  installs today, but not because the verification code is missing — it exists
  and runs (see "Release manifest signature verification" below). It stops
  because `MANIFEST_PUBLIC_KEY` is pinned to `None`, so
  `verify_release_manifest` answers `NoTrustAnchor` and refuses every manifest,
  genuine or not. That is a narrower guarantee than "installs nothing" and
  should be read as such: the barrier is one constant, and pinning a key is
  what removes it.

  **The manifest fetch happens before that refusal.** Verification cannot run
  on bytes it does not have, so a build with `RHODIZ_UPDATE_URL` set does make
  an outbound request to that host and read up to `MAX_MANIFEST_BYTES` from it
  before refusing. With the variable unset — the default — provisioning fails
  at the first step and opens no connection at all.

  **The state write is the swap.** An update imports into the distro slot that
  is *not* serving, so the runtime the operator is using keeps running through
  the whole pipeline. The write that records the new release also records its
  slot, and that single write is the moment the staged distro becomes live:
  everything before it leaves the operator running exactly what they were
  running, and a failure anywhere above it changes nothing they can observe.
  It comes last for a second reason too — persisting first would record a
  release as installed while its runtime was still unprovisioned, and because
  the anti-rollback floor only ever rises, that record would raise the floor
  permanently on behalf of a broken distro.

  The target slot is derived from **whether any state exists**, not from
  inverting the active slot. Those two agree everywhere except the case that
  matters: with no state file nothing is serving, `active_slot()` answers the
  primary slot for want of anything else to say, and inverting it would send a
  first install into the secondary slot and leave the primary permanently
  empty.

  Every distro-scoped step in the pipeline — import, `wsl.conf`, terminate and
  the Docker install — takes the slot as a parameter, so none can reach the
  live release by omission. The Docker install is the one with the most to
  lose: it runs as root inside the distro, so naming the primary distro
  unconditionally would have mutated the release still in use in order to
  provision its replacement.

  **Reclaim happens only after the swap is durable**, and its failure does not
  fail a provision that has already succeeded — the leftover distro is named in
  the success `detail` instead, because a distro the broker meant to remove and
  did not is something the operator has to be able to see. One case is recorded
  rather than closed: a *first* install that fails after its import leaves a
  distro and no state file, so a retry meets a name collision that the update
  path clears. Clearing it would need a vector that names a slot directly,
  which is the inversion `wsl_discard_inactive_args` exists to prevent.

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

## Release manifest signature verification

`broker-core/src/manifest.rs` implements the detached-signature check that the
provisioning gate above defers to. The design rule it exists to satisfy is an
ordering one: the broker **SHALL** verify the signature over the **exact bytes**
of the manifest **before** parsing any URL, version or digest, and parsing first
is a design failure rather than an optimisation.

That ordering is enforced by the type system, not by a comment. Verification
returns `VerifiedManifestBytes`, which wraps a private field, is constructed
nowhere but inside the verifier, and is the only handle that carries manifest
bytes out of the module. A parser that wants bytes must first hold one, and it
cannot fabricate one — so "parse before verify" is a compile error rather than a
review comment. `as_bytes` hands back the bytes the signature actually covered,
not a re-encoding: normalising before parsing would mean parsing something the
signature never authenticated.

- **Algorithm: Ed25519**, deterministic and parameter-free. Decisively for this
  use, verification needs **no ASN.1/DER decoding** — a DER-framed scheme would
  put a parser *inside* the verifier, reintroducing at the signature layer the
  exact hazard this module exists to eliminate.
- **`verify_strict`, never `verify`.** Both forms reject a non-canonical scalar
  (`s >= L`), so scalar malleability is closed on either path. What the strict
  form adds is the check on the order of `R` and `A`: the cofactorless equation
  accepts a signature whose `R` is a small-order point and whose `s` the key
  holder chose to match; the strict form refuses it.
- **Explicit `is_weak()` rejection** in front of it, as defence in depth:
  `VerifyingKey::from_bytes` *accepts* the all-zero key, so key construction is
  not a filter on its own.
- **Bounded input** at `MAX_MANIFEST_BYTES` (64 KiB). Verification hashes the
  whole input, so an unbounded read would let whoever serves the file choose how
  much work the broker does. The bound is inclusive, and tests pin both sides of
  it so an off-by-one fails rather than silently shifting the limit.
- **Seven refusal variants, no "verified with warnings" state.** Operator text
  is deliberately coarse: it says what was refused, never how far verification
  got or what the bytes looked like.

**No release signing key exists yet, and that is recorded honestly rather than
papered over.** `MANIFEST_PUBLIC_KEY` is `None`, so `verify_release_manifest`
refuses **every** input with `NoTrustAnchor`. Pinning a placeholder key would be
strictly worse than refusing: it would look like a configured trust anchor while
authenticating nothing real. The generic `verify_manifest_with_key` primitive is
complete and exercised, so the cryptography is ready before the key ceremony,
not after it.

When a key is issued it is pinned **at compile time**. It must never be read
from beside the manifest or from any file the update path can replace: an
attacker able to swap the manifest could then swap the key that authenticates
it, and the signature would verify perfectly against the attacker's own key.

**Test evidence: 13 tests, anchored on external authority.** Two RFC 8032
section 7.1 vectors are used, re-derived from an independent implementation —
signing a fixture with the same library that verifies it would prove only
self-consistency, not that the library implements Ed25519. TEST 1 signs the
*empty* message, so rather than being discarded it serves as the case proving
the emptiness guard runs **ahead of** the cryptography: a cryptographically
valid input that is still refused. The rest cover a flipped message bit, a
flipped signature bit, a genuine signature under the wrong key, the all-zero
small-order key, short and long keys and signatures, and both sides of the size
bound. `tests/security-contract.test.ts` pins the structural properties — the
private field, `verify_strict`, the `is_weak` check, the size bound and the
`None` anchor — against drift; each assertion was mutation-checked to confirm it
fails when the property is removed.

**Uncertified:** this is pure cryptography with no I/O, so it is fully exercised
on Linux — but nothing here has verified a *real* release manifest, because none
exists. Manifest schema, transport, anti-rollback (`release_sequence`) and digest
pinning are separate obligations that this module does not implement and must not
be read as covering.

## Update channel — what the broker trusts about where bytes come from

The broker has no compiled-in production update URL. The base comes from the
`RHODIZ_UPDATE_URL` environment variable, with **no default**: unset, provisioning
fails before any socket is opened. That is the shipping configuration today, so a
default build makes no outbound connection at all.

Three URLs are derived from that base by string concatenation — `/release.json`,
`/release.json.sig` and `/rootfs.tar.gz`. The following properties of that
arrangement are gaps, recorded here because they are not visible from the code
that reads them:

- **The scheme is not constrained.** Nothing rejects `http://`, a bare host or a
  path that is not a URL at all; whatever the variable holds is handed to
  `reqwest`. This is deliberate for now — the end-to-end test in the plan serves
  a fixture manifest from a local HTTP server, and a hard `https://` check would
  make that test impossible to write. It is defensible only because integrity is
  supposed to rest on the detached signature rather than on the transport, and
  that argument is **not yet valid in practice**: with `MANIFEST_PUBLIC_KEY` at
  `None`, nothing is authenticated, so today the only thing preventing a hostile
  update is that every manifest is refused. A scheme constraint belongs in the
  same change that pins the key, not after it.
- **The tarball URL is a producer-side convention, not a signed field.** The
  manifest schema carries no URL for the rootfs; `download_rootfs` assumes
  `rootfs.tar.gz` sits beside `release.json`. So the location of the largest
  artefact in the pipeline is not covered by the signature.
- **The base is read from the environment a second time** for the rootfs fetch,
  independently of the read that located the manifest. Nothing signed ties the
  two fetches together.
- **No rootfs digest is verified.** `download_rootfs` compares the tarball hash
  against `compose_sha256`, which is the digest of the *compose file*. The schema
  has no `rootfs_sha256` field to compare against. The comparison is left in
  place so the gap stays visible in the code; closing it is a change to what
  counts as a valid signed manifest, not a change to the download path.
- **The provisioned runtime installs packages the manifest never names.** After
  the rootfs is imported, `DOCKER_PROVISION_SCRIPT` adds Docker's own apt
  repository inside the distro and installs `docker-ce` and the Compose plugin
  from it. Those packages are whatever that repository serves at provisioning
  time — unpinned, unversioned and outside the signed manifest, which covers the
  rootfs and nothing past it. Two machines provisioned from the same signed
  release can therefore end up with different container engines. Closing this
  means pinning the versions or shipping them inside the rootfs; it is not
  closed by anything in the signature path.
- **The rootfs is bounded only by a sanity ceiling.** `MAX_ROOTFS_BYTES` (8 GiB)
  exists so a server that streams without end is refused rather than filling the
  volume. It is an order of magnitude above any plausible rootfs and expresses no
  expectation about size, because the manifest declares none.

What the transport path *does* enforce is bounded reads. The manifest and its
signature are read through a ceiling applied at the read itself, not after
buffering, and both carry a connect deadline and a whole-request deadline —
safe for them because both are small and fixed-size. The rootfs carries a
connect deadline only: a whole-request deadline would cap how large a rootfs may
legitimately be or how slow a link may be, and the blocking client exposes no
per-read inactivity timeout to use instead. **A server that trickles bytes
indefinitely therefore still stalls provisioning**, bounded only by the ceiling
above and interruptible only by closing the app.

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

Manifest verification adds a **verification-only** Ed25519 tree to `broker-core`:
`ed25519-dalek 3.0.0` with `default-features = false`, pulling `curve25519-dalek`,
`ed25519`, `signature`, `sha2 0.11`, `digest 0.11`, `block-buffer`, `crypto-common`,
`hybrid-array`, `cpufeatures`, `subtle` and `curve25519-dalek-derive`. No `rand`
(the broker never generates keys), no `alloc`, no `serde` on the crypto crates.
RustSec reports **no advisory against any of them**; the seven warnings above are
unchanged by this addition. Two consequences are recorded rather than hidden:
`sha2` and `digest` now appear at **two major versions each** in the lockfile
(0.10 via Tauri, 0.11 via the crypto tree), which is duplication rather than a
conflict; and `fiat-crypto` appears in the lockfile but in **neither** the host
nor the Windows build graph, being a target-conditional backend of
`curve25519-dalek`. Adopting `ed25519-dalek` also raised `broker-core`'s declared
`rust-version` to 1.85; the pinned workspace toolchain is 1.98.1, so this is a
truthful declaration rather than a constraint anything has to satisfy at build
time.

The five Windows warnings are upstream maintenance warnings, not known vulnerability advisories. Tauri 2.11.5 currently depends through `tauri-utils 2.9.3`, which requires the `urlpattern 0.3` line. They remain tracked debt and must be reevaluated on each Tauri update; they are not described as resolved.

## Local certification evidence

- Oxlint: 0 warnings / 0 errors.
- Vitest: 25/25 PASS.
- Executable TypeScript/React coverage: 100% statements, branches, functions and
  lines (24/24, 10/10, 14/14, 22/22). The previous checkpoint recorded 91.66%
  statements with `src/runtime/bridge.ts:68-69` uncovered — the callback
  `listenProvisioningProgress` hands to Tauri's `listen`. That gap is closed by
  a test that mocks `listen`, drives the captured handler directly, and asserts
  the three things the renderer depends on: the event name matches the one the
  broker emits, the payload arrives unwrapped rather than inside the Tauri
  envelope, and the unlisten handle is passed through so a component unmounting
  mid-provision can detach.
- Production renderer build: PASS.
- Rust broker-core: 87 unit + 2 integration = 89/89 PASS, 0 doc-tests.
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
  capability manifest against the actual permission set. It compiles every
  `#[cfg(windows)]` path.

  **How the Windows resource step was satisfied, and what that costs.** The
  build script (`tauri-winres`) shells out to `llvm-rc`, which is not installed
  in this environment. An earlier checkpoint of this document said the step used
  a user-space LLVM 20 installation; on this checkpoint that is not true, and
  the difference is worth stating rather than quietly inheriting. What ran was a
  local stub on `PATH`, outside the repository, which parses `/FO` and truncates
  the named output file to zero bytes. It is sound *only* for a type check:
  `cargo check` never links, so the `.res` is produced and never read. It means
  the version block, icon and manifest resources were **not** compiled, and no
  claim about them is supported by this run. A real resource compiler is needed
  before the packaging path can be certified — which is Windows-host work in any
  case.
- `cargo clippy --target x86_64-pc-windows-msvc -- -D warnings` over the whole
  workspace. This caught one real finding on the lifecycle lock file
  (`create` without explicit truncate behaviour, fixed by `.truncate(false)`)
  that no warning-tolerant check had surfaced before.
- `cargo clippy -p rhodiz-harness-broker-core --all-targets -- -D warnings` on
  the host, covering all pure logic including the new verify classification.

### Windows CI result, and which commit it actually covers

The `Windows Tauri check` job ran `npm run verify:windows` on a native
`windows-latest` runner for commit `85dd1e0` of this branch (workflow runs
35680565349 `push` and 35680567833 `pull_request`) and **passed**.

That SHA is named rather than described as "this commit" on purpose. A CI
result belongs to the commit it ran against and to no other, so a phrase that
follows the checkout is a claim that silently becomes false on the next push.
Any commit after `85dd1e0` is covered by this section only once its own run is
green and this paragraph names it — including the commit that carries this
paragraph, whose own run had not started when it was written.

The script is a `&&` chain, so `verify:portable`, `check:tauri` and `clippy:tauri` all succeeded there; `clippy:tauri` with
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

## Merge gate to `main` — closed until all three conditions hold

Nothing in this repository merges to `main` unless **all** of these are true,
because the stack enters `main` in order and a green cross-check is not runtime
certification:

1. **Flujo inquebrantable**: `npm run verify:portable` green end to end on the
   candidate SHA, plus `check:tauri` and `clippy:tauri` with warnings denied —
   the gate that actually compiles `broker.rs`.
   Current status: ✅ green — last full run on SHA `85dd1e0`.
2. **CI verde**: all checks green on the candidate SHA for every PR in the
   stack. Current status: ✅ PR #7 green on `85dd1e0`, its current head at the
   time of writing.
3. **E2E verde**: plan section 10 tasks (clean Windows 11 host, WSL
   absent/present/outdated paths, reboot recovery, Docker failure, digests,
   loopback under VPN, rollback, uninstall, installer + signing, full Windows
   E2E) passed on an approved Windows test environment. **Sección 10 prohíbe
   terminantemente evidencia Linux.** Current status: ❌ — no Windows host
   exists in this environment; nothing in section 10 has run.

Final acceptance additionally requires the plan (11.7): **autorización
explícita del operador** before release.

Enforcement: the operator's standing order is that a merge to `main` with the
E2E condition unsatisfied is a violation. This section exists so the gate is a
recorded matter of fact, not a remembered instruction.
