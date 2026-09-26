import { createContext, useCallback, useContext, useMemo, useState } from 'react'
import type { ReactNode } from 'react'

/** Next's typed-routes brand. Opal writes `href as Route`; here any path is one. */
export type Route = string

interface RouterValue {
  pathname: string
  push: (href: Route) => void
  replace: (href: Route) => void
  back: () => void
}

const RouterContext = createContext<RouterValue | null>(null)

/** Fallback for a tree rendered outside the provider — inert, never throwing,
 *  because Opal calls these hooks from presentational components that a test
 *  may render in isolation. */
const DETACHED: RouterValue = {
  pathname: '/',
  push: () => {},
  replace: () => {},
  back: () => {},
}

/**
 * Minimal in-memory router.
 *
 * Opal reaches for `next/navigation` in three places — the sidebar highlights
 * the active tab, the settings layout offers a back action, and
 * `useContainerCenter` re-measures when the route changes. A desktop shell has
 * no URL bar, so this keeps the same API over an in-memory stack rather than
 * stubbing the hooks out: a stub would leave the sidebar unable to show which
 * tab is open, which is a visible behaviour and not an implementation detail.
 */
export function RouterProvider({
  children,
  initialPath = '/',
}: {
  children: ReactNode
  initialPath?: string
}) {
  const [stack, setStack] = useState<string[]>([initialPath])

  const push = useCallback((href: Route) => setStack((s) => [...s, href]), [])
  const replace = useCallback(
    (href: Route) => setStack((s) => [...s.slice(0, -1), href]),
    [],
  )
  const back = useCallback(() => setStack((s) => (s.length > 1 ? s.slice(0, -1) : s)), [])

  const value = useMemo<RouterValue>(
    () => ({ pathname: stack[stack.length - 1] ?? initialPath, push, replace, back }),
    [stack, initialPath, push, replace, back],
  )
  return <RouterContext.Provider value={value}>{children}</RouterContext.Provider>
}

export function useRouter() {
  return useContext(RouterContext) ?? DETACHED
}

export function usePathname() {
  return (useContext(RouterContext) ?? DETACHED).pathname
}
