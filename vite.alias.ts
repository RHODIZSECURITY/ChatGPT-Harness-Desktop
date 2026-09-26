import { fileURLToPath } from 'node:url'

const here = (relative: string) => fileURLToPath(new URL(relative, import.meta.url))

/**
 * Module aliases, defined once because vite.config.ts and vitest.config.ts both
 * need them and a build that resolves `@opal` while the tests do not is a
 * failure mode that only shows up after the change is already written.
 *
 * Opal's own source uses the `@opal/*` spelling for both its TS and its CSS
 * imports. Keeping it is what lets the vendored tree stay byte-identical to
 * onyx-foss (PROVENANCE.md) instead of needing every import rewritten. The
 * `next/*` entries resolve to the small in-memory router in src/design/
 * next-shim, for the same reason.
 */
export const aliases = {
  '@opal': here('./src/design/opal'),
  // Opal declares this as a package `imports` subpath. Vendored source has no
  // package.json to declare it, so the mapping moves here rather than into an
  // edit of the 40 stylesheets that reference it.
  '#reference.css': here('./src/design/opal/_reference.css'),
  // Opal imports the compiled halves of @onyx-ai/shared by subpath. Both are
  // generated from the vendored token JSON by scripts/build-tokens.mjs.
  '@onyx-ai/shared/tokens.css': here('./src/design/tokens.css'),
  '@onyx-ai/shared/typography.css': here('./src/design/typography.css'),
  '@onyx-ai/shared/contracts': here('./src/design/shared/contracts/index.ts'),
  'next/navigation': here('./src/design/next-shim/navigation.tsx'),
  'next/link': here('./src/design/next-shim/link.tsx'),
  next: here('./src/design/next-shim/index.ts'),
}

/** Vendored third-party source. Excluded from coverage: counting it would put
 *  507 files this project did not write into a number that exists to describe
 *  the files it did. */
export const VENDORED = ['src/design/opal/**', 'src/design/shared/**']
