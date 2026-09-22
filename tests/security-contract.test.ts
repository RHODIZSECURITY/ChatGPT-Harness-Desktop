import { readFile } from 'node:fs/promises'
import { expect, test } from 'vitest'

// Every assertion below is a source-text contract: `indexOf` offsets and `$`
// anchors both change meaning under CRLF, and the Windows CI runner checks out
// with CRLF. Normalise once, here, so no individual test has to remember.
const read = async (relative: string) =>
  (await readFile(new URL('../' + relative, import.meta.url), 'utf8')).replace(/\r\n/g, '\n')

test('Tauri shell is local-only with a non-null CSP and production devtools disabled', async () => {
  const config = JSON.parse(await read('src-tauri/tauri.conf.json'))
  expect(config.identifier).toBe('com.rhodiz.harness.desktop')
  expect(config.app.security.csp).toContain("default-src 'self'")
  expect(config.app.security.csp).not.toContain('https:')
  expect(config.app.windows[0].devtools).toBe(false)
  expect(config.build.devUrl).toBe('http://127.0.0.1:1420')

  const capability = JSON.parse(await read('src-tauri/capabilities/default.json'))
  expect(capability.windows).toEqual(['main'])
  expect(capability.local).toBe(true)
  expect(capability.platforms).toEqual(['windows'])
  expect(capability.permissions).toEqual([
    'allow-runtime-status',
    'allow-runtime-provision',
    'allow-runtime-start',
    'allow-runtime-stop',
    'allow-runtime-verify',
    'allow-runtime-repair',
    'allow-runtime-logs',
  ])
  expect(JSON.stringify(capability)).not.toContain('core:default')
})

test('native broker exposes no generic shell or process plugin', async () => {
  const cargo = await read('src-tauri/Cargo.toml')
  const lib = await read('src-tauri/src/lib.rs')
  const broker = await read('src-tauri/src/broker.rs')
  const core = await read('src-tauri/broker-core/src/lib.rs')
  const build = await read('src-tauri/build.rs')
  expect(cargo).not.toContain('tauri-plugin-shell')
  expect(cargo).not.toContain('tauri-plugin-log')
  for (const command of [
    'broker::runtime_status',
    'broker::runtime_provision',
    'broker::runtime_start',
    'broker::runtime_stop',
    'broker::runtime_verify',
    'broker::runtime_repair',
    'broker::runtime_logs',
  ]) expect(lib).toContain(command)
  expect(lib).not.toContain('shell')
  expect(core).toContain('pub const WSL_EXE: &str = "wsl.exe"')
  expect(core).toContain('pub const WSL_STATUS_ARGS: [&str; 1] = ["--status"]')
  expect(core).toContain('pub const WSL_VERSION_ARGS: [&str; 1] = ["--version"]')
  expect(core).toContain('RUNTIME_BOOTSTRAP_UNIT: &str = "rhodiz-harness-bootstrap.service"')
  expect(core).toContain('pub const MAX_LOG_LINES: u16 = 500')
  expect(core).toContain('signed runtime manifest verification is required')
  expect(broker).toContain('runtime_logs(lines: Option<u16>)')
  expect(broker).not.toMatch(
    /runtime_(?:start|stop|provision|verify|repair)\([^)]*String/,
  )
  for (const command of [
    '"runtime_status"',
    '"runtime_provision"',
    '"runtime_start"',
    '"runtime_stop"',
    '"runtime_verify"',
    '"runtime_repair"',
    '"runtime_logs"',
  ]) expect(build).toContain(command)
})

test('the broker resolves wsl.exe from an absolute system path, never from PATH', async () => {
  const broker = await read('src-tauri/src/broker.rs')
  expect(broker).toContain('env::var_os("SystemRoot")')
  expect(broker).toContain('.join("System32").join(WSL_EXE)')
  // A bare Command::new(WSL_EXE) would resolve through PATH, which is ambient
  // authority the fail-closed baseline does not grant.
  expect(broker).not.toContain('Command::new(WSL_EXE)')
})

test('lifecycle mutations are serialized and never flash a console window', async () => {
  const broker = await read('src-tauri/src/broker.rs')
  const core = await read('src-tauri/broker-core/src/lib.rs')
  expect(broker).toContain('.creation_flags(CREATE_NO_WINDOW)')

  // Every mutating command goes through the lock, provision included: it does
  // no work today, but a call site that is already correct cannot be forgotten
  // the way a comment can.
  for (const operation of ['Provision', 'Start', 'Stop', 'Repair']) {
    expect(broker).toContain(`with_lifecycle_lock(RuntimeOperation::${operation}`)
  }

  // Repair is the only mutation that runs two spawns under one lock, and verify
  // deliberately holds no lock at all: pinning both so a later edit cannot
  // quietly swap them.
  expect(broker).toContain('with_lifecycle_lock(RuntimeOperation::Repair, platform_repair)')
  expect(broker).not.toContain('with_lifecycle_lock(RuntimeOperation::Verify')

  // An in-process Mutex cannot see a second copy of the application, and
  // nothing here prevents one. Denying all sharing on the lock file is what
  // actually keeps two processes off the same systemd unit.
  expect(broker).toContain('.share_mode(0)')
  expect(broker).toContain('LIFECYCLE_LOCK_FILE')
  expect(core).toContain('pub const LIFECYCLE_LOCK_FILE')

  // Every command that waits on wsl.exe must leave the UI thread, or the window
  // freezes for up to COMMAND_TIMEOUT_SECS.
  for (const command of [
    'runtime_status',
    'runtime_provision',
    'runtime_start',
    'runtime_stop',
    'runtime_verify',
    'runtime_repair',
    'runtime_logs',
  ]) {
    expect(broker).toMatch(
      new RegExp(String.raw`#\[tauri::command\(async\)\]\s*pub fn ${command}\(`),
    )
  }
})

test('a gate compiles the native broker with warnings denied', async () => {
  const pkg = JSON.parse(await read('package.json'))
  // verify:portable never compiles broker.rs - clippy:rust is scoped to
  // broker-core and the app crate does not build on a plain Linux host. Without
  // this gate a dead lock or a dead import reaches CI as a tolerated warning.
  expect(pkg.scripts['clippy:tauri']).toContain('-D warnings')
  expect(pkg.scripts['clippy:tauri']).not.toContain('-p ')
  expect(pkg.scripts['verify:windows']).toContain('clippy:tauri')
})

test('verify is a read-only exit-code probe and repair is bounded to one unit', async () => {
  const core = await read('src-tauri/broker-core/src/lib.rs')
  const broker = await read('src-tauri/src/broker.rs')
  // Classification is by exit code; stdout of wsl.exe is never parsed.
  expect(core).toContain('pub const SYSTEMCTL_INACTIVE_EXIT_CODE: i32 = 3')
  expect(core).toContain('pub fn classify_unit_probe(outcome: CommandOutcome)')
  expect(core).toContain('pub const UNPROBED_COMPONENTS: [&str; 5]')
  // Repair resets a latched failure and restarts the unit, nothing more.
  expect(broker).toContain('wsl_repair_reset_args(slot)')
  expect(broker).toContain('wsl_repair_restart_args(slot)')
  // Both spawns are aimed at the same slot, read once. Resolving the slot
  // twice would let a swap land between them and restart a unit in a distro
  // the reset never touched.
  expect(broker).toContain('let slot = match active_slot() {')
  expect(core).toContain('"reset-failed"')
  expect(core).toContain('"restart"')
})

test('the update stages into the inactive slot and destroys nothing until the swap is durable', async () => {
  const core = await read('src-tauri/broker-core/src/lib.rs')
  const broker = await read('src-tauri/src/broker.rs')

  // The slot to destroy is always derived from the slot that is live, never
  // named. A vector that accepted the name to remove would be one
  // argument-passing mistake away from unregistering the running runtime.
  expect(core).toContain('pub const fn wsl_discard_inactive_args(active: DistroSlot)')
  expect(core).toContain('["--unregister", active.other().distro_name()]')

  // A first install is decided by the *presence* of state, not by inverting
  // the active slot. `active_slot()` answers INITIAL when there is no state,
  // so `.other()` would send a first install into the secondary slot and
  // leave the primary one permanently empty.
  expect(broker).toContain(
    'let target_slot = previous_slot.map_or(DistroSlot::INITIAL, DistroSlot::other);',
  )
  // Asserted against code only: the comment above that derivation names the
  // rejected form in order to explain it, and would otherwise trip this.
  const brokerCode = broker
    .split('\n')
    .filter((line) => !line.trimStart().startsWith('//'))
    .join('\n')
  expect(brokerCode).not.toContain('active_slot().other()')

  // The superseded distro is reclaimed only after the swap is on disk.
  // Reclaiming first would destroy the runtime the operator is still using to
  // serve a provision that can still fail.
  const persistAt = broker.indexOf('persist_installed_release(&new_installed, target_slot)')
  const reclaimAt = broker.indexOf('wsl_discard_inactive_args(target_slot)')
  expect(persistAt).toBeGreaterThan(-1)
  expect(reclaimAt).toBeGreaterThan(persistAt)
})

test('every distro-scoped provisioning step names the staged slot, including the Docker install', async () => {
  const core = await read('src-tauri/broker-core/src/lib.rs')
  const broker = await read('src-tauri/src/broker.rs')

  // Provisioning runs four commands against a distro. All four take the slot
  // as a parameter, so none of them can be aimed at the live release by
  // omission -- which is what the Docker install did while it named the
  // primary distro unconditionally.
  for (const call of [
    'wsl_import_args(target_slot,',
    'wsl_write_conf_args(target_slot)',
    'wsl_terminate_args(target_slot)',
    'wsl_docker_provision_args(slot)',
  ]) {
    expect(broker).toContain(call)
  }
  // The Docker vector is reached through a helper, so the staged slot has to
  // arrive at that helper too.
  expect(broker).toContain('provision_docker_in_distro(target_slot)')

  // The Docker install is the one step that runs as root inside the distro,
  // so aiming it at the wrong slot mutates the running release rather than
  // merely failing.
  expect(core).toContain('pub fn wsl_docker_provision_args(slot: DistroSlot)')
  expect(core).toContain('slot.distro_name(),')
  expect(broker).not.toContain('MANAGED_DISTRO_NAME')

  // The script is a compile-time constant with nothing interpolated into it.
  // A format! or a push_str here would turn the one shell in the codebase
  // into an injection surface.
  expect(core).toContain('pub const DOCKER_PROVISION_SCRIPT: &str')
  const scriptStart = core.indexOf('pub const DOCKER_PROVISION_SCRIPT')
  const scriptEnd = core.indexOf('"#;', scriptStart)
  expect(scriptEnd).toBeGreaterThan(scriptStart)
  const script = core.slice(scriptStart, scriptEnd)
  expect(script).not.toContain('{}')
  expect(script).not.toContain('format!')
})

test('log redaction matches credential prefixes only at token boundaries', async () => {
  const core = await read('src-tauri/broker-core/src/lib.rs')
  expect(core).toContain('SENSITIVE_TOKEN_PREFIXES')
  expect(core).toContain('fn has_sensitive_token_prefix')
  expect(core).toContain('[REDACTED SENSITIVE LOG LINE]')
})

test('provenance anchors all three approved source repositories at exact commits', async () => {
  const provenance = await read('PROVENANCE.md')
  const pins = [
    'a0403035a20c91decadd011b907ee5b489f6788b',
    'e11c874019dbf04032cfc3476d82eba1d069a3d8',
    '2452f499a86ae215146d988e9418a481616238ad',
  ]
  for (const pin of pins) expect(provenance).toContain(pin)

  // Assert the shape of every source row, not the literal 'None yet'. Pinning
  // that string makes "nothing imported yet" a permanent invariant, so the test
  // would fail exactly when all three upstreams finally record a real import.
  const rows = provenance
    .split('\n')
    .filter((line) => pins.some((pin) => line.includes(pin)))
  expect(rows).toHaveLength(pins.length)
  for (const row of rows) {
    const cells = row.split('|').map((cell) => cell.trim()).filter(Boolean)
    expect(cells).toHaveLength(5)
    expect(cells[1]).toMatch(/^[0-9a-f]{40}$/)
    expect(cells[4].length).toBeGreaterThan(0)
  }
})
test('renderer dependency set contains no alternate backend or host shell', async () => {
  const pkg = JSON.parse(await read('package.json'))
  const names = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies })
  expect(names).not.toContain('@tauri-apps/plugin-shell')
  expect(names).not.toContain('electron')
  expect(pkg.scripts['verify:portable']).toContain('test:coverage')
})
test('Rust toolchain is pinned with the Windows target', async () => {
  const toolchain = await read('rust-toolchain.toml')
  expect(toolchain).toContain('channel = "1.98.1"')
  expect(toolchain).toContain('x86_64-pc-windows-msvc')
})

test('visible naming is canonical while stable protocol compatibility remains explicit', async () => {
  const legacyVisibleName = ['RHODIZ', 'Arnes'].join('-')
  const authoredPaths = [
    'README.md',
    '.project/Context.md',
    '.project/ProjectMemory.md',
    'docs/SECURITY_BASELINE.md',
    'package.json',
    'src-tauri/Cargo.toml',
    'src-tauri/tauri.conf.json',
    'src-tauri/broker-core/src/lib.rs',
  ]
  const authored = await Promise.all(authoredPaths.map(read))
  expect(authored.join('\n')).not.toContain(legacyVisibleName)
  expect(authored[0]).toContain('RHODIZ-Harness')
  expect(authored[1]).toContain('`rhodiz-arnes`')
  expect(authored[2]).toContain('RHODIZ-Harness')
  expect(authored[7]).toContain('MANAGED_DISTRO_NAME: &str = "RHODIZ-Harness"')
})

test('GitHub CI is pinned and certifies portable plus Windows gates', async () => {
  const workflow = await read('.github/workflows/ci.yml')
  expect(workflow).toContain('actions/checkout@11d5960a326750d5838078e36cf38b85af677262')
  expect(workflow).toContain('actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020')
  expect(workflow).toContain("node-version: '22.23.2'")
  expect(workflow).toContain('rustup toolchain install 1.98.1')
  expect(workflow).toContain('npm audit --audit-level=high')
  expect(workflow).toContain('cargo audit --file src-tauri/Cargo.lock')
  expect(workflow).toContain('npm run verify:portable')
  expect(workflow).toContain('npm run verify:windows')
  expect(workflow).not.toContain('|| true')
})

// The provisioning preflight is the only place in the broker that reads
// wsl.exe stdout for anything but logs, because `wsl --version` writes
// UTF-16LE. Pin that it stays a narrow, fail-closed exception: the decode
// refuses a truncated capture, and the preflight never turns an unreadable
// version into permission to provision.
test('the WSL version probe is a bounded, fail-closed exception to exit-code classification', async () => {
  const broker = await read('src-tauri/src/broker.rs')
  const core = await read('src-tauri/broker-core/src/lib.rs')
  expect(broker).toContain('fn read_wsl_version() -> Option<(u32, u32, u32)>')
  expect(broker).toContain('|| capture.truncated')
  expect(broker).toContain('provisioning_preflight(status, version)')
  // Provisioning still refuses to create anything on every branch.
  expect(core).toMatch(
    /pub fn provisioning_preflight[\s\S]*?state: OperationState::Blocked/,
  )
  // An undeterminable version is a refusal, never an assumed-sufficient pass.
  expect(core).toContain(
    'the installed WSL version could not be determined; refusing to provision',
  )
})

// Every Provision result must carry a non-empty detail explaining the block.
// The renderer must not be forced to guess why provisioning is blocked.
test('every Provision outcome carries a detail string', async () => {
  const core = await read('src-tauri/broker-core/src/lib.rs')
  // provisioning_preflight uses a match expression assigning to a `detail` binding,
  // then constructs RuntimeOperationResult { detail, ... }. Count Some(...) arms
  // inside the match (5 branches set detail: Some(...), 1 returns early).
  const preflightFn = core.slice(core.indexOf('pub fn provisioning_preflight'))
  const someCount = (preflightFn.match(/=> Some\(/g) || []).length
  expect(someCount).toBeGreaterThanOrEqual(5)
})

// design.md §105 is explicit that the broker verifies the signature over the
// exact manifest bytes *before* parsing any URL, version or digest, and that
// parsing first is a design failure rather than an optimisation. The module
// enforces that ordering through its types, not a comment, so pin the
// structural properties that make the enforcement real: a comment can be
// deleted without any test noticing, but these cannot.
test('manifest signature verification is enforced before any parse', async () => {
  const source = await read('src-tauri/broker-core/src/manifest.rs')

  // Assert against production code only. `mod tests` names every constant and
  // every error variant checked below, so reading the whole file would let a
  // deleted guard keep passing on the strength of the test that covered it.
  // Assert against production code only. If tests live inline, strip them;
  // if they've been extracted to a separate file, the whole source is
  // production code — but assert it contains no test attributes, so the
  // recorte never silently reverts to analysing test code.
  const testModule = source.indexOf('#[cfg(test)]')
  const manifest = testModule > 0 ? source.slice(0, testModule) : source
  if (testModule < 0) {
    expect(manifest).not.toMatch(/#\[test\]/)
  }

  // The handle that carries verified bytes has a private field and no
  // exported constructor, so a caller cannot fabricate one and parse first.
  // Widening the tuple field to `pub` would silently make the ordering
  // optional again.
  expect(manifest).toContain("pub struct VerifiedManifestBytes<'a>(&'a [u8])")
  expect(manifest).not.toMatch(/pub struct VerifiedManifestBytes<'a>\(pub /)

  // verify_strict, not verify: the strict form rejects small-order keys and
  // torsion components, which is what removes signature malleability.
  expect(manifest).toMatch(/\.verify_strict\(/)
  expect(manifest.replace(/\/\/.*$/gm, "")).not.toMatch(/[^_]\.verify\(/)

  // VerifyingKey::from_bytes accepts the all-zero key, so key construction is
  // not a filter on its own.
  expect(manifest).toContain('if key.is_weak()')

  // The verifier hashes the whole input, so an unbounded read would let
  // whoever serves the manifest choose how much work the broker does. Pin the
  // guard, not the variant name: a bare name would still be present if the
  // `if` that enforces it were deleted.
  expect(manifest).toContain('pub const MAX_MANIFEST_BYTES: usize = 64 * 1024')
  expect(manifest).toMatch(
    /if manifest\.len\(\) > MAX_MANIFEST_BYTES \{\s*return Err\(ManifestVerifyError::ManifestTooLarge\);/,
  )

  // The ordering guards exist to keep unbounded input away from the hash
  // function. Moving them below verify_strict would turn them into dead
  // code — verify_strict would hash the full input first.
  const sizeGuardPos = manifest.indexOf('if manifest.len() > MAX_MANIFEST_BYTES')
  const emptyGuardPos = manifest.indexOf('if manifest.is_empty()')
  const verifyStrictPos = manifest.indexOf('.verify_strict(')
  expect(sizeGuardPos).toBeGreaterThan(0)
  expect(emptyGuardPos).toBeGreaterThan(0)
  expect(verifyStrictPos).toBeGreaterThan(0)
  expect(emptyGuardPos).toBeLessThan(verifyStrictPos)
  expect(sizeGuardPos).toBeLessThan(verifyStrictPos)

  // No release signing key exists yet. `None` keeps the release path refusing
  // every input; a placeholder would look like a configured trust anchor
  // while authenticating nothing.
  // The anchor is a compile-time constant, never loaded from a file or an
  // environment variable. Pin that, not the exact initializer — "= None" will
  // change the day a real key is pinned, and the contract should survive it.
  expect(manifest).toMatch(
    /pub const MANIFEST_PUBLIC_KEY: Option<\[u8; MANIFEST_PUBLIC_KEY_LEN\]> = (None|Some\(\[)/,
  )
  expect(manifest).not.toMatch(/include_bytes!|include_str!|std::env|option_env!/)
  expect(manifest).toContain('return Err(ManifestVerifyError::NoTrustAnchor)')

  // The verified handle hands back the exact bytes the signature covered.
  // Re-encoding or normalising here would mean parsing something the
  // signature never authenticated.
  expect(manifest).toMatch(/pub\s+fn\s+as_bytes\(&self\)\s*->\s*&'a\s*\[u8\]/)

  // Choosing the key is choosing the trust anchor. The key-taking verifier
  // stays private to the module so no caller can verify against an anchor of
  // its own and still end up holding a VerifiedManifestBytes; the pinned
  // constant is the only way in.
  // Pinned by line-start anchor so pub(crate) would also be caught.
  expect(manifest).toMatch(/^fn verify_manifest_with_key</m)
  expect(manifest).not.toMatch(/^pub[\s(].*fn verify_manifest_with_key</m)

  // The module is private and re-exported narrowly, so the crate root offers
  // exactly one entry point. Re-exporting the key-taking verifier, or making
  // the module public again, would put the anchor back in the caller's hands.
  const core = await read('src-tauri/broker-core/src/lib.rs')
  expect(core).toMatch(/^mod manifest;\s*(?:\/\/.*)?$/m)
  expect(core).not.toMatch(/^pub[\s(].*mod manifest;$/m)
  const reExportMatch = core.match(/pub use manifest::\{([^}]*)\}/)
  expect(reExportMatch).not.toBeNull()
  const reExportBlock = reExportMatch![1]
  expect(reExportBlock).toContain('verify_release_manifest')
  expect(reExportBlock).not.toContain('verify_manifest_with_key')
})

// The provisioning pipeline draws security conclusions from the order its
// steps run in and from the difference between "no state" and "unreadable
// state". Both are properties of control flow that no type enforces, so pin
// them here: each assertion below corresponds to a way the pipeline has
// already been wrong once.
test('provisioning fails closed on unreadable state and installs before it records success', async () => {
  const broker = await read('src-tauri/src/broker.rs')

  // Only a missing file may read as "nothing installed". Any other read or
  // parse failure has to surface, because `decide_update` takes the absence
  // of state as the absence of an anti-rollback floor -- so a swallowed
  // error would let an arbitrarily old signed release install.
  expect(broker).toContain('fn load_persisted_state() -> Result<Option<PersistedState>, String>')
  expect(broker).toContain('Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None)')

  // ...and the caller has to act on that error rather than defaulting. Assert
  // on the span between the load and the decision: a `Failed` return has to
  // sit inside it, which is what stops provisioning before a floor-less
  // decision can be made.
  const loadAt = broker.indexOf('match load_persisted_state() {')
  const decideAt = broker.indexOf('decide_update(&manifest, installed)')
  expect(loadAt).toBeGreaterThan(-1)
  expect(decideAt).toBeGreaterThan(loadAt)
  expect(broker.slice(loadAt, decideAt)).toContain('state: OperationState::Failed')

  // Docker is installed before state is persisted. Persisting first would
  // record a release as installed while its runtime is still unprovisioned,
  // and the recorded sequence would raise the rollback floor on behalf of a
  // distro with no working Docker in it -- permanently, since the floor only
  // ever rises.
  const dockerAt = broker.indexOf('ProvisioningStep::InstallDocker')
  const persistAt = broker.indexOf('ProvisioningStep::PersistState')
  expect(dockerAt).toBeGreaterThan(-1)
  expect(persistAt).toBeGreaterThan(dockerAt)

  // Every broker-owned path hangs off one fallible accessor. Unwrapping the
  // variable instead panics inside a Tauri command handler, where the
  // renderer gets a dropped IPC call and no reason for it.
  expect(broker).toContain('fn local_app_data() -> Result<PathBuf, String>')
  expect(broker).not.toMatch(/var_os\("LOCALAPPDATA"\)\s*\)?\s*\.\s*unwrap\(\)/)
  expect(broker).not.toMatch(/var_os\("LOCALAPPDATA"\)\.expect\(/)
})

// Every byte the broker pulls off the network arrives before anything has
// verified it, and arrives at a length the server chooses. Both facts have to
// be handled at the read itself: a ceiling checked afterwards runs only once
// the body is already in memory, and a request with no deadline never reaches
// the check at all. Neither is expressible as a type, so pin them here.
test('network reads are bounded in size and time before anything verifies them', async () => {
  const broker = await read('src-tauri/src/broker.rs')

  // No client without deadlines. `Client::new()` is the constructor that
  // gives you one, so its absence is what makes the builders below the only
  // way to get a client at all.
  expect(broker).not.toContain('reqwest::blocking::Client::new()')

  // Each builder is checked on its own body. Asserting the connect deadline
  // appears somewhere in the file would pass with one client still missing
  // it, which is exactly the shape this has to rule out.
  const clientBody = (name: string) => {
    const at = broker.indexOf(`fn ${name}() -> Result<reqwest::blocking::Client, String> {`)
    expect(at, `${name} is missing`).toBeGreaterThan(-1)
    const end = broker.indexOf('\n}', at)
    expect(end).toBeGreaterThan(at)
    return broker.slice(at, end)
  }
  const manifestClient = clientBody('manifest_client')
  const rootfsClient = clientBody('rootfs_client')
  for (const body of [manifestClient, rootfsClient]) {
    expect(body).toContain('.connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))')
  }

  // The manifest and its signature are small and fixed-size, so they also
  // carry a whole-request deadline. The rootfs deliberately does not: a total
  // timeout there would cap how large a legitimate rootfs may be, and
  // reqwest's blocking builder offers no per-read inactivity timeout to use
  // in its place.
  expect(manifestClient).toContain('.timeout(Duration::from_secs(MANIFEST_HTTP_TIMEOUT_SECS))')
  expect(rootfsClient).not.toMatch(/\.timeout\(/)

  // The manifest and signature bodies are bounded where they are read, not
  // where they are checked. `verify_release_manifest` does enforce
  // MAX_MANIFEST_BYTES, but only on a Vec that is already in memory -- an
  // endless response body exhausts RAM before it ever runs.
  expect(broker).toContain('read_body_bounded(manifest_resp, MAX_MANIFEST_BYTES, "manifest")')
  expect(broker).toContain('read_body_bounded(sig_resp, MANIFEST_SIGNATURE_LEN, "signature")')

  // ...and that helper refuses rather than truncates. A prefix of a signed
  // document is not a shorter signed document.
  const bodyBoundedAt = broker.indexOf('fn read_body_bounded(')
  expect(bodyBoundedAt).toBeGreaterThan(-1)
  expect(broker.slice(bodyBoundedAt, bodyBoundedAt + 600)).toContain('exceeds its {limit}-byte ceiling')

  // The rootfs is streamed to disk under a ceiling, never buffered whole. The
  // digest comparison cannot stand in for either: it runs only after the
  // write has finished.
  expect(broker).toContain('io::copy(&mut (&mut resp).take(MAX_ROOTFS_BYTES + 1), &mut file)')
  expect(broker).not.toMatch(/resp\s*\n?\s*\.bytes\(\)/)

  // Hitting the ceiling removes the partial file before returning. Scoped to
  // the ceiling branch itself: the digest-mismatch branch further down does
  // its own cleanup, and a slice wide enough to include it would pass on that
  // one alone.
  const ceilingAt = broker.indexOf('if copied > MAX_ROOTFS_BYTES {')
  expect(ceilingAt).toBeGreaterThan(-1)
  const ceilingBranch = broker.slice(ceilingAt, broker.indexOf('return Err(', ceilingAt))
  expect(ceilingBranch).toContain('fs::remove_file(&tarball_path)')
})

// The provisioning pipeline's safety content is its order. Verification sits
// between the bytes arriving and anything being done with them, so every
// effect that costs something -- a multi-hundred-megabyte download, a distro
// import, a package install as root -- has to sit below it. Nothing in the
// type system says so: the guarantee is that the statements appear in that
// sequence inside one function, which is exactly the kind of thing a later
// edit reorders without noticing.
test('nothing is downloaded or imported before the manifest signature verifies', async () => {
  const broker = await read('src-tauri/src/broker.rs')

  // The twelve steps run in the order the enum declares them. Emitting them
  // out of order would not break provisioning, which is the problem: the UI
  // would narrate a sequence the broker is not performing, and the progress
  // events are the only window an operator has into a pipeline that
  // otherwise runs silently for minutes.
  const ORDER = [
    'FetchManifest',
    'Verify',
    'Parse',
    'Decide',
    'ResolveBundle',
    'DownloadRootfs',
    'VerifyDigest',
    'ImportWsl',
    'ConfigureSystemd',
    'RestartWsl',
    'InstallDocker',
    'PersistState',
  ]
  // The trailing comma matters: `ProvisioningStep::Verify` is a prefix of
  // `ProvisioningStep::VerifyDigest`, and matching the prefix would silently
  // compare a step against itself.
  const positions = ORDER.map((step) => {
    const at = broker.indexOf(`ProvisioningStep::${step},`)
    expect(at, `${step} is never emitted`).toBeGreaterThan(-1)
    return at
  })
  for (let i = 1; i < positions.length; i += 1) {
    expect(positions[i]!, `${ORDER[i]} must be emitted after ${ORDER[i - 1]}`).toBeGreaterThan(
      positions[i - 1]!,
    )
  }

  // The verify call, not the import at the top of the file: `use ... {
  // verify_release_manifest, ... }` sits above every line below and would
  // make each of these comparisons trivially true.
  const verifyAt = broker.indexOf('match verify_release_manifest(&manifest_bytes, &signature_bytes)')
  expect(verifyAt).toBeGreaterThan(-1)

  // Everything that acts on the manifest's contents happens below the
  // verification of those contents. A prefetch added "while we verify", or a
  // bundle resolved early to show the user a size, would put an
  // attacker-chosen URL in front of the signature check.
  for (const effect of [
    'download_rootfs(&bundle)',
    'wsl_import_args(target_slot,',
    'wsl_write_conf_args(target_slot)',
    'wsl_terminate_args(target_slot)',
    'provision_docker_in_distro(target_slot)',
    'wsl_discard_inactive_args(',
  ]) {
    const at = broker.indexOf(effect)
    expect(at, `${effect} is missing`).toBeGreaterThan(-1)
    expect(at, `${effect} must not run before the signature verifies`).toBeGreaterThan(verifyAt)
  }

  // A failed verification returns; it does not fall through with a warning.
  // Scoped to the span between the verify and the next step so a `Failed`
  // return belonging to some later stage cannot satisfy it.
  const verifyBlock = broker.slice(verifyAt, broker.indexOf('ProvisioningStep::Parse,'))
  expect(verifyBlock).toContain('state: OperationState::Failed')
  expect(verifyBlock).toContain('release manifest verification failed')
})

// The Rust enum is the publisher and the TypeScript union is the subscriber,
// and serde's rename rule is the only thing connecting them. Nothing fails to
// compile when they drift: Rust emits a step the renderer's union does not
// name, or the renderer offers a case that nothing will ever send, and the
// progress display is wrong in a way that only shows up during a real
// provision on a real Windows machine.
test('the Rust provisioning steps and the renderer union name the same steps in the same order', async () => {
  const broker = await read('src-tauri/src/broker.rs')
  const types = await read('src/runtime/types.ts')

  const enumAt = broker.indexOf('pub enum ProvisioningStep {')
  expect(enumAt).toBeGreaterThan(-1)
  const enumBody = broker.slice(enumAt, broker.indexOf('\n}', enumAt))
  const declared = [...enumBody.matchAll(/^ {4}(\w+),$/gm)].map((m) => m[1]!)
  expect(declared.length).toBeGreaterThan(0)

  // The wire names are derived from the variants rather than written out a
  // second time, so this compares the two languages instead of comparing two
  // copies of the same hand-written list.
  expect(broker.slice(Math.max(0, enumAt - 200), enumAt)).toContain(
    '#[serde(rename_all = "snake_case")]',
  )
  const wire = declared.map((variant) =>
    variant.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase(),
  )

  const unionAt = types.indexOf('export type ProvisioningStep =')
  expect(unionAt).toBeGreaterThan(-1)
  const unionEnd = types.indexOf('\n\n', unionAt)
  expect(unionEnd).toBeGreaterThan(unionAt)
  const union = [...types.slice(unionAt, unionEnd).matchAll(/'([a-z0-9_]+)'/g)].map((m) => m[1]!)

  expect(union).toEqual(wire)
})
