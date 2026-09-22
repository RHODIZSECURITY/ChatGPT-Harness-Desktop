import { readFile } from 'node:fs/promises'
import { expect, test } from 'vitest'

const read = (relative: string) => readFile(new URL('../' + relative, import.meta.url), 'utf8')

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
  expect(broker).toContain('let slot = active_slot();')
  expect(core).toContain('"reset-failed"')
  expect(core).toContain('"restart"')
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
