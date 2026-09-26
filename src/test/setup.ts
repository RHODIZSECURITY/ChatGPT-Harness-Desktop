import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'
import '@testing-library/jest-dom/vitest'

// Testing Library only auto-cleans when vitest runs with `globals: true`, and
// this project does not. Without it a second render in the same file sees the
// first one's DOM and every `getByRole` becomes ambiguous — which surfaces as
// "found multiple elements", a failure that reads like a component bug.
afterEach(cleanup)

/**
 * jsdom implements neither of these, and Opal's layout primitives use both —
 * the sidebar measures itself with ResizeObserver, and its responsive
 * breakpoints read matchMedia.
 *
 * These are stubs, not shims: they report nothing and observe nothing, so a
 * test can render the shell but must not assert on measured geometry. That is
 * the honest boundary — jsdom has no layout engine, so a richer fake would be
 * inventing numbers rather than reporting them.
 */
if (!('ResizeObserver' in globalThis)) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver
}

if (!('matchMedia' in globalThis)) {
  globalThis.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof matchMedia
}

/**
 * Radix's Select scrolls the highlighted option into view as the list opens.
 * jsdom has no layout and therefore no `scrollIntoView`, so without this the
 * menu throws before it can be read from — a failure in the test environment
 * that looks exactly like a broken control.
 *
 * Same boundary as the stubs above: the scroll moves nothing, so a test may
 * open the list and assert on what it contains, never on where it sits.
 */

if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {}
}

/**
 * Radix's Select tracks pointer capture on its trigger while the menu opens.
 * jsdom implements neither `hasPointerCapture` nor its setters, so the open
 * throws an unhandled error before the list mounts — a failure in the test
 * environment that looks exactly like options that do not exist.
 *
 * Same boundary as the stubs above: capture moves nothing in jsdom, so a
 * test may open the list and assert on what it contains, never on pointer
 * routing. `hasPointerCapture` reports false rather than throwing, which is
 * what a pointer that was never captured would answer.
 */

if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false
}

if (!Element.prototype.setPointerCapture) {
  Element.prototype.setPointerCapture = () => {}
}

if (!Element.prototype.releasePointerCapture) {
  Element.prototype.releasePointerCapture = () => {}
}
