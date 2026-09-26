/**
 * Theme resolution.
 *
 * Deliberately free of React and of any DOM global beyond the element it is
 * handed, so the three decisions that matter — what a stored value means, what
 * a preference resolves to, and what the document ends up carrying — are
 * testable as pure functions rather than through a rendered tree.
 *
 * There is exactly one mechanism: a `light` class on `<html>`. The generated
 * token sheet emits the dark set under `:root` and the light set under
 * `.light` at equal specificity, so the later rule wins and the class is
 * enough. `color-scheme` follows it in CSS (`src/index.css`) rather than being
 * written from here — two writers for one piece of state is two things that
 * can disagree.
 */

/** Same string as the bootstrap in `index.html`. A test pins that they match. */
export const THEME_STORAGE_KEY = 'rhodiz-harness.theme'

/** The class the generated token sheet keys its light set on. */
export const LIGHT_CLASS = 'light'

/**
 * Queried for *light*, not dark, because dark is this application's default:
 * a browser that answers nothing at all leaves the dark set in place, which is
 * the same thing it would do if the query were absent.
 */
export const SYSTEM_LIGHT_QUERY = '(prefers-color-scheme: light)'

export const THEME_PREFERENCES = ['dark', 'light', 'system'] as const

/**
 * `erasableSyntaxOnly` forbids a TypeScript `enum`, so the union is derived
 * from the tuple — which is also what lets the control iterate the options
 * without a second list to keep in step.
 */
export type ThemePreference = (typeof THEME_PREFERENCES)[number]

/** What a preference becomes once the system has been asked. */
export type ResolvedTheme = 'dark' | 'light'

export function isThemePreference(value: unknown): value is ThemePreference {
  return (
    typeof value === 'string' &&
    (THEME_PREFERENCES as readonly string[]).includes(value)
  )
}

export function resolveTheme(
  preference: ThemePreference,
  systemPrefersLight: boolean,
): ResolvedTheme {
  if (preference === 'system') return systemPrefersLight ? 'light' : 'dark'
  return preference
}

/** Toggle rather than add/remove: the call site states the end state, not a delta. */
export function applyTheme(root: Element, resolved: ResolvedTheme): void {
  root.classList.toggle(LIGHT_CLASS, resolved === 'light')
}

/**
 * Anything that is not a preference this build knows about reads as `system`.
 *
 * That covers a key written by an older build, a key edited by hand, and a
 * storage that throws on access — a reasonable browser setting, not an
 * exceptional one. None of those are worth failing a render over: the value
 * being recovered is a preference, and losing it degrades to following the
 * operating system, which is what this application does before anyone has
 * expressed one.
 */
export function readStoredPreference(storage: Pick<Storage, 'getItem'>): ThemePreference {
  let stored: string | null
  try {
    stored = storage.getItem(THEME_STORAGE_KEY)
  } catch {
    return 'system'
  }
  return isThemePreference(stored) ? stored : 'system'
}

/**
 * Best effort, and the caller is told so by the return value rather than by a
 * comment: storage can be full or switched off, and a theme click is not a
 * reason to tear down the tree. When this returns `false` the choice still
 * applies for the rest of the session — it just will not survive a restart.
 */
export function writeStoredPreference(
  storage: Pick<Storage, 'setItem'>,
  preference: ThemePreference,
): boolean {
  try {
    storage.setItem(THEME_STORAGE_KEY, preference)
    return true
  } catch {
    return false
  }
}
