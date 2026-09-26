import { describe, expect, it } from 'vitest'
import {
  applyTheme,
  isThemePreference,
  readStoredPreference,
  resolveTheme,
  writeStoredPreference,
  LIGHT_CLASS,
  THEME_PREFERENCES,
  THEME_STORAGE_KEY,
  type ThemePreference,
} from './theme'

describe('resolveTheme', () => {
  it.each([
    ['dark', false, 'dark'],
    ['dark', true, 'dark'],
    ['light', false, 'light'],
    ['light', true, 'light'],
    ['system', false, 'dark'],
    ['system', true, 'light'],
  ] as const)('%s with systemPrefersLight=%s resolves to %s', (preference, light, expected) => {
    expect(resolveTheme(preference, light)).toBe(expected)
  })

  it('does not let the system override an explicit choice', () => {
    // The one guarantee a two-state toggle plus an auto option has to make:
    // picking a side means the OS stops being consulted.
    expect(resolveTheme('dark', true)).toBe('dark')
    expect(resolveTheme('light', false)).toBe('light')
  })
})

describe('isThemePreference', () => {
  it('accepts every preference the control can offer', () => {
    for (const preference of THEME_PREFERENCES) {
      expect(isThemePreference(preference)).toBe(true)
    }
  })

  it.each([null, undefined, '', 'Dark', 'auto', 0, {}])('rejects %o', (value) => {
    expect(isThemePreference(value)).toBe(false)
  })
})

describe('readStoredPreference', () => {
  it('returns what was stored when it is a preference', () => {
    for (const preference of THEME_PREFERENCES) {
      expect(readStoredPreference({ getItem: () => preference })).toBe(preference)
    }
  })

  it.each([
    ['nothing stored', null],
    ['a value from another build', 'midnight'],
    ['an empty string', ''],
  ])('falls back to system with %s', (_label, stored) => {
    expect(readStoredPreference({ getItem: () => stored })).toBe('system')
  })

  it('falls back to system when storage throws rather than failing the render', () => {
    expect(
      readStoredPreference({
        getItem: () => {
          throw new Error('storage is disabled')
        },
      }),
    ).toBe('system')
  })
})

describe('writeStoredPreference', () => {
  it('writes under the shared key and reports success', () => {
    const written: Array<[string, string]> = []
    expect(
      writeStoredPreference({ setItem: (key, value) => written.push([key, value]) }, 'light'),
    ).toBe(true)
    expect(written).toEqual([[THEME_STORAGE_KEY, 'light']])
  })

  it('reports failure rather than throwing when storage refuses', () => {
    expect(
      writeStoredPreference(
        {
          setItem: () => {
            throw new Error('quota exceeded')
          },
        },
        'dark',
      ),
    ).toBe(false)
  })
})

describe('applyTheme', () => {
  it('states the end result rather than a delta', () => {
    const root = document.createElement('html')

    applyTheme(root, 'light')
    expect(root.classList.contains(LIGHT_CLASS)).toBe(true)
    applyTheme(root, 'light')
    expect(root.classList.contains(LIGHT_CLASS)).toBe(true)

    applyTheme(root, 'dark')
    expect(root.classList.contains(LIGHT_CLASS)).toBe(false)
    applyTheme(root, 'dark')
    expect(root.classList.contains(LIGHT_CLASS)).toBe(false)
  })

  it('leaves classes it did not put there alone', () => {
    const root = document.createElement('html')
    root.classList.add('some-other-class')
    applyTheme(root, 'light')
    applyTheme(root, 'dark')
    expect(root.classList.contains('some-other-class')).toBe(true)
  })
})

describe('THEME_PREFERENCES', () => {
  it('is the single list the control and the union both come from', () => {
    // If this changes, the label map in ThemeControl stops typechecking —
    // which is the point of deriving the union from the tuple.
    const all: ThemePreference[] = [...THEME_PREFERENCES]
    expect(all).toEqual(['dark', 'light', 'system'])
  })
})
