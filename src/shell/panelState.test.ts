import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { readPanelFlag, writePanelFlag } from './panelState'

const KEY = 'test.panel'

beforeEach(() => localStorage.clear())
afterEach(() => vi.unstubAllGlobals())

/**
 * Replaces `localStorage` with a host that refuses to hand it over at all —
 * which is what a browser with storage switched off does. The property has to
 * be redefined rather than assigned: `globalThis.localStorage` is a getter,
 * and assigning to it is silently ignored.
 */
function installHostileStorage() {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    get() {
      throw new Error('storage is disabled')
    },
  })
  return () => {
    if (original === undefined) delete (globalThis as { localStorage?: unknown }).localStorage
    else Object.defineProperty(globalThis, 'localStorage', original)
  }
}

describe('readPanelFlag', () => {
  it('returns the fallback when nothing was ever stored', () => {
    expect(readPanelFlag(KEY, true)).toBe(true)
    expect(readPanelFlag(KEY, false)).toBe(false)
  })

  it('reads back both sides of a stored choice', () => {
    writePanelFlag(KEY, false)
    expect(readPanelFlag(KEY, true)).toBe(false)
    writePanelFlag(KEY, true)
    expect(readPanelFlag(KEY, false)).toBe(true)
  })

  it('treats anything that is not one of the two words as unset', () => {
    // Not a parse: `Boolean('false')` is `true`, and a panel that opens
    // because the stored value was garbage is worse than one that opens
    // because nothing was stored.
    for (const junk of ['', 'TRUE', '1', 'null', '{"open":true}']) {
      localStorage.setItem(KEY, junk)
      expect(readPanelFlag(KEY, false), junk).toBe(false)
      expect(readPanelFlag(KEY, true), junk).toBe(true)
    }
  })

  it('falls back rather than throwing on a host with no storage', () => {
    const restore = installHostileStorage()
    try {
      expect(readPanelFlag(KEY, true)).toBe(true)
    } finally {
      restore()
    }
  })
})

describe('writePanelFlag', () => {
  it('reports the write it could not make', () => {
    const restore = installHostileStorage()
    try {
      // The return value is the whole point: silence would leave the caller
      // unable to tell a stored preference from a forgotten one.
      expect(writePanelFlag(KEY, true)).toBe(false)
    } finally {
      restore()
    }
    expect(writePanelFlag(KEY, true)).toBe(true)
  })
})
