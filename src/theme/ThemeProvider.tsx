import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react'
import {
  applyTheme,
  readStoredPreference,
  resolveTheme,
  writeStoredPreference,
  SYSTEM_LIGHT_QUERY,
  type ResolvedTheme,
  type ThemePreference,
} from './theme'

interface ThemeContextValue {
  /** What the user chose. `system` is a choice, not the absence of one. */
  preference: ThemePreference
  /** What that choice currently means. `system` collapses to one of the two. */
  resolved: ResolvedTheme
  setPreference: (preference: ThemePreference) => void
}

const ThemeContext = createContext<ThemeContextValue | null>(null)

/**
 * Reading `localStorage` — not just calling it — can throw when storage is
 * switched off, so even acquiring the object is guarded. A null return means
 * this session follows the operating system and forgets the choice on exit,
 * which is the same degradation `readStoredPreference` describes.
 */
function safeStorage(): Storage | null {
  try {
    return localStorage
  } catch {
    return null
  }
}

function systemPrefersLight(): boolean {
  return typeof matchMedia === 'function' && matchMedia(SYSTEM_LIGHT_QUERY).matches
}

/**
 * Owns the theme for the running application.
 *
 * The first paint is not this component's job — `index.html` has already put
 * the right class on `<html>` synchronously, before any module loads, so a
 * light-theme user never sees a dark frame. What lives here is everything
 * *after* that: the choice, its persistence, and the listener that keeps
 * `system` honest while the window stays open. Resolving the media query once
 * at boot would make `system` mean "whatever the system was when you launched".
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(() => {
    const storage = safeStorage()
    return storage === null ? 'system' : readStoredPreference(storage)
  })
  const [prefersLight, setPrefersLight] = useState(systemPrefersLight)

  useEffect(() => {
    if (typeof matchMedia !== 'function') return
    const query = matchMedia(SYSTEM_LIGHT_QUERY)
    const onChange = (event: MediaQueryListEvent) => setPrefersLight(event.matches)
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [])

  const resolved = resolveTheme(preference, prefersLight)

  useEffect(() => {
    applyTheme(document.documentElement, resolved)
  }, [resolved])

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next)
    const storage = safeStorage()
    if (storage !== null) writeStoredPreference(storage, next)
  }, [])

  return (
    <ThemeContext.Provider value={{ preference, resolved, setPreference }}>
      {children}
    </ThemeContext.Provider>
  )
}

/**
 * Throws rather than returning a default. A control rendered outside the
 * provider would otherwise show a theme it cannot change, and look like it
 * works.
 */
export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext)
  if (value === null) throw new Error('useTheme must be used inside a ThemeProvider')
  return value
}
