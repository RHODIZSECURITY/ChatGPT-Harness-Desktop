# RHODIZ Harness Desktop — Source Provenance

Every adapted/copied upstream module must be recorded here before merge.
Pins are immutable provenance anchors, never floating build inputs.

| Source | Exact commit | License | Classification | Imported code |
| --- | --- | --- | --- | --- |
| RHODIZSECURITY/openhands | a0403035a20c91decadd011b907ee5b489f6788b | MIT | ADAPT | None yet |
| RHODIZSECURITY/onyx-foss | e11c874019dbf04032cfc3476d82eba1d069a3d8 | MIT, **except `web/src/ee/` and `backend/ee/`** (see below) | ADOPT/ADAPT | `web/lib/opal` and `web/lib/shared` → `src/design/` |
| RHODIZSECURITY/librechat | 2452f499a86ae215146d988e9418a481616238ad | MIT | ADAPT | None yet |

## Import rule

For every imported component add: upstream path, exact commit, copyright/license,
local destination, modification summary, security review outcome, and tests.
Backends/agent runtimes from these sources are not execution authorities.
## onyx-foss is not MIT end to end

The table above said `MIT` without qualification. That was checked against the
repository root `LICENSE`, and the root is MIT. It is not the whole tree.

`web/src/ee/` and `backend/ee/` carry their own **Onyx Enterprise License**
(`web/src/ee/LICENSE`), which requires a paid Onyx Enterprise License for
production use and states:

> it is forbidden to copy, merge, publish, distribute, sublicense, and/or sell
> the Software

Measured at the pinned commit: 51 of 1373 TypeScript files under `web/src`
(3.7%), confined to `app/ee/admin` (performance analytics, standard answers),
`app/ee/agents`, and `EEFeatureRedirect.tsx`. The design system and component
library are outside it — verified, not assumed: nothing in `web/lib/opal`,
`web/lib/shared` or `web/src/components/ui` imports from either `ee` path.

**Nothing under an `ee/` path may be copied into this repository.** The row is
annotated rather than downgraded because the parts this project draws on are
genuinely MIT.

## Imported: onyx design tokens

| field | value |
| --- | --- |
| Upstream path | `web/lib/shared/tokens/{primitives,semantic-dark,semantic-light,size,typography,shadow}.json` |
| Exact commit | `e11c874019dbf04032cfc3476d82eba1d069a3d8` |
| License | MIT — declared in `web/lib/shared/package.json` |
| Local destination | `src/design/tokens/` |
| Modification | **None.** Byte-identical; see the checksums below. |

```
91d21753e37c698cc716ea09e3337d2aecde589b34136640e58a587c905b4b03  primitives.json
4dfead35fb709fe365179ab89f862cbe349d511866b93322866d93c93b264a4d  semantic-dark.json
a88a1dbf431ff4614348591db8099647679d7beaecf2dfa19b91dd71d7c99ec5  semantic-light.json
fed4f1e0635595328d6cc2f69a37b7e21b2501b61889ca3674972361b19aa0fa  shadow.json
f7883890d822ef71669612bca63ba041867f4a5847e887cccb04e669344178c8  size.json
8cbc4fa8cb80bb4d6561eff2e407a60bdc48df9546794f35c0637613b64d17cf  typography.json
912bfa809ae280b47642cb3cd1cd9ce4331f6bec71d801a5d4cd513c862cbe1d  typography-presets.json
```

**Security review.** The imported files are data, not code: JSON literals of
colour, size and typography values with single-hop `{alias}` references into
`primitives.json`. They define no behaviour and are never evaluated. They are
read at build time by `scripts/build-tokens.mjs`, which this project owns and
which rejects an alias that is cyclic or that resolves to no primitive rather
than emitting it. Upstream compiles the same inputs with Style Dictionary v4;
that dependency is deliberately not taken, because a build-time tree is supply
chain surface in an application that ships signed releases, and the inputs stay
byte-identical either way.

**Tests.** `tests/security-contract.test.ts` pins that the vendored tokens stay
unmodified and that no `ee/` path is ever vendored. The shell renders from the
generated utilities under `npm run verify:portable` (27 tests, 100% coverage).

**Not imported.** `@onyx-ai/opal`'s `logos/` subpath ships third-party brand
marks (Anthropic, OpenAI, Slack, GitHub and others) for nominative use only,
per its `NOTICE.md`. Those marks are not this project's to redistribute and are
excluded. Opal's components are not imported yet either — only the tokens they
are built on.

## Adaptations that import no code

**Semantic token naming.** An earlier revision of `src/index.css` took its
elevation-based surface naming from openhands. That layer has been replaced by
the onyx tokens above and the openhands naming is gone; the note is kept so the
history is not silently rewritten.

`Imported code` remains `None yet` for openhands and librechat, and that is
exact.

## Imported: Opal, the onyx component library

| field | value |
| --- | --- |
| Upstream path | `web/lib/opal/src` (minus `logos/`), `web/lib/opal/tailwind-preset.cjs`, `web/lib/shared/src/contracts` |
| Exact commit | `e11c874019dbf04032cfc3476d82eba1d069a3d8` |
| License | MIT — declared in `web/lib/opal/package.json`, with the trademark carve-out in its `NOTICE.md` |
| Local destination | `src/design/opal/`, `src/design/tailwind-preset.cjs`, `src/design/shared/contracts/` |
| Tree digest | `c1b09ed4fa048eb77c42a87d3044c886cdfbc76059167e52fd9c0c3adb179646` over 501 files |

Reproduce the digest with `node scripts/vendor-digest.mjs`. It is one sha256
over a sorted `<sha256>  <path>` manifest of the whole vendored tree, because
per-file checksums are reviewable for six token files and decoration for five
hundred component files.

**Modifications.** Three, all subtractive, none to a retained file's bytes:

1. `logos/` (98 files) is not vendored. It ships third-party brand marks —
   Anthropic, OpenAI, Slack, GitHub and others — for nominative use only, per
   Opal's `NOTICE.md`. They are not this project's to redistribute. Nothing in
   the retained tree imports them; the only two mentions are prose in doc
   comments.
2. 66 `*.stories.tsx` removed. Storybook is not a dependency here and their
   imports would fail the build.
3. 10 `*.test.tsx` removed. They resolve `@tests/setup/test-utils` and
   `@/lib/sidebar/utils` from the onyx web app, which is not vendored.

**Not modified: the source itself.** Opal is written against Next, and this is
a Vite application. Rather than patch the eight files that import `next/link`
or `next/navigation` — which would end the byte-identity the digest attests —
those specifiers resolve through `vite.alias.ts` to `src/design/next-shim`, a
small in-memory router this project owns and tests. `@opal/*`,
`#reference.css` and the two `@onyx-ai/shared/*.css` subpaths are aliased the
same way and for the same reason.

The vendored tree is typechecked under `tsconfig.vendor.json`, which carries
the settings it was written against rather than this repository's stricter
ones, for the same reason: satisfying `verbatimModuleSyntax` and
`noUnusedLocals` would mean editing 27 upstream files. Code this project
authors, `src/design/next-shim` included, stays strict.

**Security review.** Opal is presentational: components, layouts, icons and
hooks over radix, dnd-kit and tanstack-table. Searched, not assumed: the
vendored tree contains no `fetch`, `XMLHttpRequest`, `WebSocket`,
`EventSource`, `sendBeacon`, dynamic `import()`, `eval`,
`dangerouslySetInnerHTML`, `localStorage` or `document.cookie`, and invokes
nothing over the Tauri bridge.

It touches browser storage once. `layouts/sidebar/components.tsx` writes the
sidebar's scroll offset to `sessionStorage` under a `scrollKey` the caller
supplies, and reads it back on navigation. That is a scroll position and
nothing else, it is per-tab and does not outlive the window, and the key is
not attacker-chosen — but the sentence above would be false without it, so it
is recorded rather than rounded off. The 22 peer dependencies it needs are a real increase in supply-chain
surface for an application that ships signed releases, and are recorded in
`package.json` at the majors Opal declares; three were initially resolved to a
wrong major by `npm install` and corrected (`@tanstack/react-table` to ^8,
`react-markdown` to ^9, `tailwind-merge` to ^2).

**Tests.** `tests/security-contract.test.ts` pins the per-file token checksums
and the tree digest, and fails if any `ee` path appears under `src/`. The
router shim has its own suite. `npm run verify:portable` covers the rest.
