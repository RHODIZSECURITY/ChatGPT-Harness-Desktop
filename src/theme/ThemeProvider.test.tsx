import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ThemeControl from '../shell/ThemeControl'
import { ThemeProvider, useTheme } from './ThemeProvider'
import { LIGHT_CLASS, SYSTEM_LIGHT_QUERY, THEME_STORAGE_KEY } from './theme'

/**
 * `src/test/setup.ts` stubs `matchMedia` because jsdom has none, but that stub
 * is inert on purpose: it reports `false` and observes nothing. `system` mode
 * is the one behaviour that needs a media query that can actually change, so
 * this file installs a controllable one and drives it.
 */
function installMatchMedia(initiallyLight: boolean) {
  const listeners = new Set<(event: MediaQueryListEvent) => void>()
  let matches = initiallyLight

  vi.stubGlobal(
    'matchMedia',
    vi.fn((query: string) => ({
      get matches() {
        return query === SYSTEM_LIGHT_QUERY && matches
      },
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) =>
        listeners.add(listener),
      removeEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) =>
        listeners.delete(listener),
      dispatchEvent: () => false,
    })),
  )

  return {
    set(next: boolean) {
      matches = next
      for (const listener of listeners) {
        listener({ matches: next } as MediaQueryListEvent)
      }
    },
    get listenerCount() {
      return listeners.size
    },
  }
}

const isLight = () => document.documentElement.classList.contains(LIGHT_CLASS)

beforeEach(() => {
  localStorage.clear()
  document.documentElement.classList.remove(LIGHT_CLASS)
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('ThemeProvider', () => {
  it('starts on the system preference when nothing was ever chosen', () => {
    installMatchMedia(true)
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    expect(screen.getByRole('radio', { name: 'Auto' })).toBeChecked()
    expect(isLight()).toBe(true)
  })

  it('restores a stored choice over the system preference', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'dark')
    installMatchMedia(true)
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked()
    expect(isLight()).toBe(false)
  })

  it('applies and persists a choice made from the control', async () => {
    installMatchMedia(false)
    const user = userEvent.setup()
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    await user.click(screen.getByRole('radio', { name: 'Light' }))

    expect(isLight()).toBe(true)
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light')
  })

  it('follows the system while it changes, rather than reading it once at boot', async () => {
    const media = installMatchMedia(false)
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    expect(isLight()).toBe(false)
    await vi.waitFor(() => expect(media.listenerCount).toBe(1))

    media.set(true)
    await vi.waitFor(() => expect(isLight()).toBe(true))

    media.set(false)
    await vi.waitFor(() => expect(isLight()).toBe(false))
  })

  it('stops following the system once a side is chosen', async () => {
    const media = installMatchMedia(false)
    const user = userEvent.setup()
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    await user.click(screen.getByRole('radio', { name: 'Dark' }))
    media.set(true)

    // The listener stays subscribed — going back to Auto has to work without a
    // remount — but its answer no longer reaches the document.
    await vi.waitFor(() => expect(isLight()).toBe(false))

    await user.click(screen.getByRole('radio', { name: 'Auto' }))
    await vi.waitFor(() => expect(isLight()).toBe(true))
  })

  it('drops its listener on unmount', async () => {
    const media = installMatchMedia(false)
    const { unmount } = render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    await vi.waitFor(() => expect(media.listenerCount).toBe(1))
    unmount()
    expect(media.listenerCount).toBe(0)
  })

  it('works on a host where reading localStorage throws', async () => {
    // Not a hypothetical: a browser with site data blocked throws on the
    // property access itself, before any method is called. The theme still has
    // to switch — it just will not be remembered.
    const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      get() {
        throw new Error('storage is disabled')
      },
    })
    try {
      installMatchMedia(false)
      const user = userEvent.setup()
      render(
        <ThemeProvider>
          <ThemeControl />
        </ThemeProvider>,
      )

      expect(screen.getByRole('radio', { name: 'Auto' })).toBeChecked()
      await user.click(screen.getByRole('radio', { name: 'Light' }))
      expect(isLight()).toBe(true)
    } finally {
      if (original === undefined) delete (globalThis as { localStorage?: unknown }).localStorage
      else Object.defineProperty(globalThis, 'localStorage', original)
    }
  })

  it('renders on a host with no matchMedia at all', () => {
    vi.stubGlobal('matchMedia', undefined)
    expect(() =>
      render(
        <ThemeProvider>
          <ThemeControl />
        </ThemeProvider>,
      ),
    ).not.toThrow()
    expect(isLight()).toBe(false)
  })
})

describe('useTheme', () => {
  it('refuses to work outside a provider rather than looking like it does', () => {
    function Orphan() {
      useTheme()
      return null
    }
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => render(<Orphan />)).toThrow(/ThemeProvider/)
    quiet.mockRestore()
  })
})

describe('ThemeControl', () => {
  it('is one labelled group of radios, not three separate controls', () => {
    installMatchMedia(false)
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    const group = screen.getByRole('group', { name: 'Theme' })
    const radios = screen.getAllByRole('radio')
    expect(radios).toHaveLength(3)
    // One `name` across the three: the browser's own roving focus depends on
    // it, and so does only ever having one of them checked.
    const names = new Set(radios.map((radio) => (radio as HTMLInputElement).name))
    expect(names.size).toBe(1)
    for (const radio of radios) expect(group).toContainElement(radio)
  })
})
