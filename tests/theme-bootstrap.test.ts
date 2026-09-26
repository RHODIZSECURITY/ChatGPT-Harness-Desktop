import { readFile } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'
import {
  readStoredPreference,
  resolveTheme,
  LIGHT_CLASS,
  SYSTEM_LIGHT_QUERY,
  THEME_PREFERENCES,
  THEME_STORAGE_KEY,
} from '../src/theme/theme'

const readSource = async (relative: string) =>
  // Normalised: a checkout with `core.autocrlf=true` would otherwise fail on
  // line endings rather than on content.
  (await readFile(new URL('../' + relative, import.meta.url), 'utf8')).replace(/\r\n/g, '\n')

/**
 * The bootstrap in `index.html` cannot import this module — it has to run
 * before the module graph exists, which is the whole reason it is there. So it
 * restates the key, the class and the query, and restates the fallback rule.
 *
 * Asserting the three strings are present would only prove they are present.
 * These tests run the bootstrap's own source against a fake document and
 * require it to reach the same answer this module reaches, for every stored
 * value and both system states — which is the thing that actually has to stay
 * true.
 */
describe('the pre-paint bootstrap in index.html', () => {
  const loadBootstrap = async () => {
    const html = await readSource('index.html')
    const match = /<script>([\s\S]*?)<\/script>/.exec(html)
    expect(match, 'index.html has no inline bootstrap script').not.toBeNull()
    return match![1]
  }

  it('restates the constants this module owns', async () => {
    const source = await loadBootstrap()
    expect(source).toContain(THEME_STORAGE_KEY)
    expect(source).toContain(SYSTEM_LIGHT_QUERY)
    expect(source).toContain(LIGHT_CLASS)
  })

  const CASES: Array<string | null> = [...THEME_PREFERENCES, null, 'midnight', '']

  it.each(
    CASES.flatMap((stored) =>
      [false, true].map((prefersLight) => [stored, prefersLight] as const),
    ),
  )('agrees with resolveTheme for stored=%o prefersLight=%s', async (stored, prefersLight) => {
    const source = await loadBootstrap()
    const classes = new Set<string>()
    const run = new Function(
      'localStorage',
      'matchMedia',
      'document',
      source,
    ) as (
      storage: { getItem: (key: string) => string | null },
      media: (query: string) => { matches: boolean },
      doc: { documentElement: { classList: { add: (name: string) => void } } },
    ) => void

    run(
      { getItem: (key) => (key === THEME_STORAGE_KEY ? stored : null) },
      (query) => ({ matches: query === SYSTEM_LIGHT_QUERY && prefersLight }),
      { documentElement: { classList: { add: (name) => classes.add(name) } } },
    )

    const expected = resolveTheme(
      readStoredPreference({ getItem: () => stored }),
      prefersLight,
    )
    expect(classes.has(LIGHT_CLASS)).toBe(expected === 'light')
  })

  it('survives a host with no storage at all', async () => {
    const source = await loadBootstrap()
    const run = new Function('localStorage', 'matchMedia', 'document', source) as (
      ...args: unknown[]
    ) => void
    const throwing = {
      getItem: () => {
        throw new Error('storage is disabled')
      },
    }
    expect(() =>
      run(throwing, () => ({ matches: false }), {
        documentElement: { classList: { add: () => {} } },
      }),
    ).not.toThrow()
  })
})
