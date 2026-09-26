/**
 * Persistence for the two panels the user can hide.
 *
 * A panel the user closed and finds open again on the next launch is a panel
 * that did not do what it was told, so the choice outlives the window. The
 * whole access is wrapped rather than only the call: reading `localStorage`
 * itself throws when storage is switched off, and a shell that cannot open
 * because of a preference is worse than a shell that forgets one. The
 * degradation is the same as the theme's — this session works, the next one
 * starts from the default.
 */

export const SIDEBAR_FOLDED_KEY = 'rhodiz-harness.sidebar-folded'
export const WORKSPACE_OPEN_KEY = 'rhodiz-harness.workspace-open'

/** Anything that is not the string `true` or `false` is treated as unset. */
export function readPanelFlag(key: string, fallback: boolean): boolean {
  try {
    const raw = localStorage.getItem(key)
    if (raw === 'true') return true
    if (raw === 'false') return false
    return fallback
  } catch {
    return fallback
  }
}

/** Returns whether the write landed, so a caller can tell silence from failure. */
export function writePanelFlag(key: string, value: boolean): boolean {
  try {
    localStorage.setItem(key, String(value))
    return true
  } catch {
    return false
  }
}
