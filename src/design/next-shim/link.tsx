import { forwardRef } from 'react'
import type { AnchorHTMLAttributes } from 'react'
import { useRouter } from './navigation'
import type { Route } from './navigation'

export type { Route }

export interface LinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  href: Route
  /** Accepted and ignored: Next prefetches routes over the network, and there
   *  is no network here. Declared so Opal's call sites type-check unchanged. */
  prefetch?: boolean
  replace?: boolean
  /** Accepted and ignored for the same reason as `prefetch`: there is no
   *  document scroll position to restore across an in-memory push. */
  scroll?: boolean
}

/**
 * `next/link` for a shell with no Next.
 *
 * Renders a real anchor so keyboard focus, middle-click and the accessibility
 * tree behave the way Opal's components assume, but routes through the
 * in-memory router instead of letting the webview navigate: a real navigation
 * inside Tauri would tear down the renderer.
 */
const Link = forwardRef<HTMLAnchorElement, LinkProps>(function Link(
  { href, prefetch: _prefetch, scroll: _scroll, replace, onClick, children, ...rest },
  ref,
) {
  const router = useRouter()
  return (
    <a
      {...rest}
      ref={ref}
      href={href}
      onClick={(event) => {
        onClick?.(event)
        if (event.defaultPrevented || event.metaKey || event.ctrlKey) return
        event.preventDefault()
        if (replace) router.replace(href)
        else router.push(href)
      }}
    >
      {children}
    </a>
  )
})

export default Link
