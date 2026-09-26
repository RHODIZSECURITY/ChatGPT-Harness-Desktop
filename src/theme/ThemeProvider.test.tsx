import { render, screen } from '@testing-library/react'
import userEvent, { type UserEvent } from '@testing-library/user-event'
import { Provider as TooltipProvider } from '@radix-ui/react-tooltip'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SidebarFoldedContext } from '@opal/layouts/sidebar/context'
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

/** The closed control: the one node that states which theme is in force. */
const trigger = () => screen.getByRole('combobox')

/**
 * Opens the list from the keyboard, then picks a row by its label.
 *
 * The keyboard is not a stylistic choice. Opening the list with a pointer
 * leaves something behind that survives unmount, and the *next* test in this
 * file then clicks the trigger and gets nothing: measured as three options on
 * the first open and zero on the second, with no error raised either time.
 * Enter on a focused trigger opens it as many times as it is asked to, and it
 * exercises the one interaction a listbox must support anyway.
 *
 * The label is matched as a prefix rather than exactly, because a row is not
 * one string: Radix keeps its own copy of the label for typeahead, and Opal
 * gives every row a description underneath.
 */
async function choose(user: UserEvent, label: string) {
  trigger().focus()
  await user.keyboard('{Enter}')
  await user.click(await screen.findByRole('option', { name: new RegExp('^' + label) }))
}

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

    expect(trigger()).toHaveTextContent('Auto')
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

    expect(trigger()).toHaveTextContent('Dark')
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

    await choose(user, 'Light')

    expect(isLight()).toBe(true)
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light')
    // The trigger reports the choice back, rather than only having applied it.
    expect(trigger()).toHaveTextContent('Light')
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

    await choose(user, 'Dark')
    media.set(true)

    // The listener stays subscribed — going back to Auto has to work without a
    // remount — but its answer no longer reaches the document.
    await vi.waitFor(() => expect(isLight()).toBe(false))

    await choose(user, 'Auto')
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

      expect(trigger()).toHaveTextContent('Auto')
      await choose(user, 'Light')
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
  it('announces both its subject and its current value', () => {
    installMatchMedia(false)
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    // Subject and value, split the way a labelled native select splits them:
    // the name says what the control is for, the content says where it stands.
    // An `aria-label` would replace that content, and the value would stop
    // being announced at all — which is the failure this asserts against.
    expect(trigger()).toHaveAccessibleName('Theme')
    expect(trigger()).toHaveTextContent('Auto')
  })

  it('offers the three preferences and nothing else', async () => {
    installMatchMedia(false)
    const user = userEvent.setup()
    render(
      <ThemeProvider>
        <ThemeControl />
      </ThemeProvider>,
    )

    trigger().focus()
    await user.keyboard('{Enter}')

    const options = await screen.findAllByRole('option')
    expect(options.map((option) => option.textContent)).toEqual([
      'DarkDarkAlways dark',
      'LightLightAlways light',
      'AutoAutoFollow the system',
    ])
    // Exactly one is current, and it is the one the trigger shows.
    const selected = options.filter((option) => option.getAttribute('aria-selected') === 'true')
    expect(selected).toHaveLength(1)
    expect(selected[0].textContent).toContain('Auto')
  })
})

describe('ThemeControl, folded', () => {
  /**
   * The real context, not a stand-in.
   *
   * `SidebarRoot` is what installs it in the application, but mounting the
   * whole sidebar here would test the layout rather than this control, and
   * `effectiveFolded` there is a function of viewport width — which jsdom
   * reports as a fixed 1024 and cannot be folded by. Rendering the provider
   * directly asserts against the same value the component reads in the app.
   */
  function renderFolded() {
    installMatchMedia(false)
    return render(
      <ThemeProvider>
        <TooltipProvider>
          <SidebarFoldedContext.Provider value={true}>
            <ThemeControl />
          </SidebarFoldedContext.Provider>
        </TooltipProvider>
      </ThemeProvider>,
    )
  }

  it('replaces the select with a control rather than removing it', () => {
    renderFolded()

    // The defect this guards against is the rail offering no way to change
    // the theme at all, which is what returning null used to do.
    expect(screen.queryByRole('combobox')).toBeNull()
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('says where it stands and what pressing it will do', () => {
    renderFolded()

    // A control with no text has one line to carry both, or the user is
    // guessing at every press.
    expect(screen.getByRole('button')).toHaveAccessibleName(
      'Theme: Auto — switch to Dark',
    )
  })

  it('advances through all three preferences and back', async () => {
    const user = userEvent.setup()
    renderFolded()
    const button = () => screen.getByRole('button')

    await user.click(button())
    expect(button()).toHaveAccessibleName('Theme: Dark — switch to Light')
    expect(isLight()).toBe(false)

    await user.click(button())
    expect(button()).toHaveAccessibleName('Theme: Light — switch to Auto')
    expect(isLight()).toBe(true)
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light')

    // Three presses from Auto returns to Auto: the cycle closes, so nothing
    // chosen here is a one-way door.
    await user.click(button())
    expect(button()).toHaveAccessibleName('Theme: Auto — switch to Dark')
    expect(isLight()).toBe(false)
  })
})
