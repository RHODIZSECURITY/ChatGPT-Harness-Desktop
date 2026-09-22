// Prints one digest over every vendored file under src/design/opal and
// src/design/shared, plus the Tailwind preset that sits beside them.
//
// Per-file checksums are the right shape for the six token JSONs, which a
// reviewer can read. They are the wrong shape for five hundred component
// files: nobody audits that list, so it becomes decoration. One digest over a
// sorted "<sha256>  <path>" manifest is auditable in the only way that matters
// here — it changes when anything under the vendored tree changes.
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync, statSync } from 'node:fs'

// Resolved from the working directory rather than import.meta.url: the test
// runner that imports this does not hand modules a file: URL, and a digest
// helper that only works when run as a CLI is a digest nobody checks.
const root = process.cwd() + '/'
export const VENDORED_PATHS = [
  'src/design/opal',
  'src/design/shared',
  'src/design/tailwind-preset.cjs',
]

function walk(relative, out = []) {
  const absolute = root + relative
  if (statSync(absolute).isDirectory()) {
    for (const entry of readdirSync(absolute).sort()) walk(`${relative}/${entry}`, out)
  } else {
    out.push(relative)
  }
  return out
}

export function vendorDigest() {
  const files = VENDORED_PATHS.flatMap((path) => walk(path)).sort()
  const manifest = files
    .map((file) => `${createHash('sha256').update(readFileSync(root + file)).digest('hex')}  ${file}`)
    .join('\n')
  return { digest: createHash('sha256').update(manifest).digest('hex'), count: files.length }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const { digest, count } = vendorDigest()
  console.log(`${digest}  ${count} files`)
}
