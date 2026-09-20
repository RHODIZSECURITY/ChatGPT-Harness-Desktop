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
  expect(capability.permissions).toEqual(['allow-runtime-status'])
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
  expect(lib).toContain('generate_handler![broker::runtime_status]')
  expect(lib).not.toContain('shell')
  expect(core).toContain('pub const WSL_EXE: &str = "wsl.exe"')
  expect(core).toContain('pub const WSL_STATUS_ARGS: [&str; 1] = ["--status"]')
  expect(broker).toContain('Command::new(WSL_EXE).args(WSL_STATUS_ARGS)')
  expect(build).toContain('commands(&["runtime_status"])')
})
test('provenance anchors all three approved source repositories at exact commits', async () => {
  const provenance = await read('PROVENANCE.md')
  expect(provenance).toContain('a0403035a20c91decadd011b907ee5b489f6788b')
  expect(provenance).toContain('e11c874019dbf04032cfc3476d82eba1d069a3d8')
  expect(provenance).toContain('2452f499a86ae215146d988e9418a481616238ad')
  expect(provenance).toContain('None yet')
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
