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

## Imported: the two typefaces this application renders in

| field | value |
| --- | --- |
| Upstream | [Hanken Grotesk](https://github.com/marcologous/hanken-grotesk) v3.013, [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) v2.211 |
| Retrieved from | Google Fonts, `css2` API with `display=swap` (family revisions v12 and v24) |
| License | SIL Open Font License 1.1 — `src/design/fonts/OFL-HankenGrotesk.txt`, `OFL-JetBrainsMono.txt` |
| Local destination | `src/design/fonts/` |
| Classification | **ADOPT.** Binary assets, byte-identical to what was downloaded. |

```
e9201eddf1d41d0b62253295d869ce3cf65768f7102b797f02c7f8c876b4a9d5  HankenGrotesk-latin.woff2
768af2923e0ab1549f1dfba0a5c8ea749c4c01f01d8e77ffaf7fcd12f57a0a24  HankenGrotesk-latin-ext.woff2
18be452724bfdc236c074ca94a249a7f41a86752c7d04ab258ce9ed5651f6a7e  JetBrainsMono-latin.woff2
79bfdab9ba467e26eea4122e6f2567e188dd8a09a8c730d501fc487c4ab99c6e  JetBrainsMono-latin-ext.woff2
```

Source URLs, in the same order:

```
https://fonts.gstatic.com/s/hankengrotesk/v12/ieVn2YZDLWuGJpnzaiwFXS9tYtpd59A.woff2
https://fonts.gstatic.com/s/hankengrotesk/v12/ieVn2YZDLWuGJpnzaiwFXS9tYtpT59CjCQ.woff2
https://fonts.gstatic.com/s/jetbrainsmono/v24/tDbV2o-flEEny0FZhsfKu5WU4xD7OwE.woff2
https://fonts.gstatic.com/s/jetbrainsmono/v24/tDbV2o-flEEny0FZhsfKu5WU4xD1OwG_TA.woff2
```

**Why these are in the repository at all.** The Tauri CSP is
`font-src 'self' data:`. A font that is not in the bundle cannot be fetched,
and an `@font-face` pointing off the machine does not fail loudly — the shell
just renders in whatever the operating system picks, differently on every
machine. Self-hosting is not a preference here, it is the only thing the
application permits.

**Why variable and not static.** `src/design/typography.css` asks for weight
450 in one preset. A static 400 file does not reject that, it snaps to 400, and
the preset silently stops being a distinct weight. Verified with fontTools
rather than assumed from the CDN: Hanken Grotesk carries `fvar` axis
`wght 100–900`, JetBrains Mono `wght 100–800`, both at 1000 units per em. Every
weight the type scale uses (400, 450, 500, 600, 700) is inside both.

**Which token each one answers.** `--font-hanken-grotesk` already names
`"Hanken Grotesk"` first, so the `@font-face` is all it needed.
`--font-dm-mono` names `"DM Mono"`, which is not bundled and cannot be fetched,
so `src/design/brand.css` repoints the token at JetBrains Mono; the token name
is generated from the pinned onyx JSON and is therefore not this project's to
rename.

**KH Teka is left where it is, deliberately.** Opal declares two `@font-face`
rules for it in `src/design/opal/styles/typography.css`, pointing at
`/fonts/KHTeka-*.otf` — a commercial face, and files that do not exist in this
bundle. The rules are inert: no typography preset references the family, so the
browser never requests them. Removing the file would mean editing the bytes of
two retained vendored files that import it (`root.css` and `_reference.css`),
which is the one thing the tree digest above exists to prevent. What is done
instead costs nothing pinned: `brand.css` points the `--font-kh-teka` *token*
at the typeface this application actually ships, so a future use of the token
renders in the product's own face rather than a system fallback.

**Security review.** Fonts are parsed by the platform's own shaping stack,
which is the same code path every page the user already opens exercises; the
exposure is the file's integrity, and that is what the checksums above pin. The
subsets are Google's latin and latin-ext slices — no Google script, no
stylesheet and no runtime call to `fonts.gstatic.com` comes with them, and the
CSP would block all three. Both licences permit redistribution and ship
alongside the binaries; neither is a Reserved Font Name build, so no renaming
obligation applies.

**Tests.** `tests/security-contract.test.ts` pins the four checksums, that both
licence files are present, that every `@font-face` resolves to a relative path
inside the bundle that exists on disk, that the CSP still forbids a remote one,
that every weight the scale asks for lies inside the bundled axes, and that
`brand.css` is imported after the generated tokens — which is the only reason
its overrides win.

## Authored here: the accent ramp

`src/design/brand.css` replaces eight of onyx's accent tokens —
`--action-selection-00..06` and `--action-text-link-05` — in both themes. No
upstream bytes are involved; the values are derived, and the derivation is
recorded so it can be re-run rather than trusted.

onyx's accent sits at OKLCH hue 262°. The blue in the RHODIZ mark measures
249°. Side by side that reads as two blues rather than one, and the accent is
the colour the eye tracks: selection, focus, links. Each step keeps its own
OKLCH lightness and chroma and changes only hue, so the ramp's internal
relationships stay the ones onyx designed; chroma is reduced where 249° at that
lightness falls outside sRGB. The rotation is done in OKLCH and not HSL because
HSL's lightness is not perceptual — a cyan-leaning blue at fixed HSL lightness
comes out visibly brighter than the indigo it replaces, which would quietly
change every contrast ratio in the shell.

Step 05 is re-fitted rather than rotated. It is the fill behind white text and
the link colour in the light theme, and at its original lightness the brand hue
lands at 4.18:1 on white, below WCAG AA. Its lightness drops from 0.577 to
0.559, which restores 4.50:1. Measured before and after:

| | on | before | after |
| --- | --- | --- | --- |
| link, dark `#397bff` → `#0086fa` | `#000000` | 5.45 | 5.79 |
| link, dark | `#1a1a1a` | 4.52 | 4.80 |
| link, light `#286df8` → `#0073ec` | `#ffffff` | 4.53 | 4.50 |
| selection-05, dark `#286df8` → `#0073ec` | `#000000` | 4.63 | 4.66 |
| white on selection-05, dark | `#0073ec` | 4.53 | 4.50 |

Nothing regresses below AA. **The neutrals are untouched** — they are onyx's
greys, they are the reason the shell reads as calm, and repainting them would
be redesigning the design system rather than branding it.

## Authored here: the sidebar mark

`src/shell/rhodiz-mark.png` is derived from `assets/brand/rhodiz-mark-1024-transparent.png`
— the mark on alpha — cropped to its own bounding box `(62, 2, 952, 904)`,
padded to a square, and resampled to 112×112. No upstream bytes; the source is
the RHODIZ brand artwork this repository already carries.

It exists because the component rendered `src-tauri/icons/64x64.png` instead.
That file is the packaged application icon and is correct as such: 4096 of 4096
pixels fully opaque, a flat tile from `#161B23` to `#090B0F`, because a taskbar
draws no backdrop of its own. Inside the window it is a hard-edged rectangle
darker than the surface behind it — a box in the dark theme, a black square in
the light one. The regression is silent: the import resolves and the image
decodes, so only a screenshot shows it.

The backdrop now travels with the component rather than being baked into the
file. The mark is silver — mean luma 178 over its opaque pixels — so it is
legible on the dark sidebar and washes out on the light one; the chip carries
the icon's own two values, fixed in both themes, which is what keeps the
sidebar mark and the taskbar icon the same object. `tests/security-contract.test.ts`
asserts the import target, the 112×112 dimensions and PNG colour type 6, so an
opaque re-export fails rather than shipping.

## Adapted, not vendored: the streaming packet protocol

| field | value |
| --- | --- |
| Upstream paths | `web/src/app/app/services/streamingModels.ts`, `web/src/app/app/services/packetUtils.ts`, `web/src/app/app/message/messageComponents/renderMessageComponent.tsx` |
| Exact commit | `e11c874019dbf04032cfc3476d82eba1d069a3d8` |
| License | MIT — outside `web/src/ee/` |
| Local destination | `src/conversation/{protocol,grouping,findRenderer}.ts`, `src/conversation/renderers.tsx` |
| Classification | **ADAPT.** No upstream bytes are present; the digest above does not cover these files and no checksum is claimed for them. |

This is the one import so far that is written rather than copied, so it is
recorded differently on purpose. A subset of a file is a different file: taking
20 of onyx's 60-odd packet types and trimming the rest would produce something
that neither matches upstream nor can be re-derived from it, and a checksum
over that would attest to nothing.

**What is kept identical: the wire strings.** `message_delta`, `bash_tool_delta`,
`coding_agent_final` and the rest carry onyx's exact values, because the value
is the protocol. Keeping them means onyx's backend, its tests and a captured
trace stay readable here as a reference for what a packet means. The TypeScript
construct differs — upstream uses an `enum`, and this repository sets
`erasableSyntaxOnly`, which forbids one — so it is a `const` object with a
derived union. That changes the declaration, not a single byte on the wire.

**What is left behind.** Search, citations, deep research, research agents,
intermediate reports, image generation, the memory tool, `fetch`/`open_url` and
the Python interpreter. Those describe a retrieval product; this application
runs an agent in a WSL distro and has no retrieval pipeline to describe.

**What is changed deliberately.** Two things:

1. Upstream's `findRenderer` is fourteen sequential `if` statements. Here the
   same precedence is an exported `DISPATCH` table, because the order is the
   behaviour — it is the only thing that decides a group holding both an answer
   and a tool packet — and a table is a value a test can assert against rather
   than control flow a test has to re-derive.
2. Upstream folds `error` packets into the reasoning renderer. Here every
   renderer ends with the group's error, so a failure that arrives inside a
   shell command is shown *with* the output that explains it rather than
   replacing it. `CustomToolRenderer` additionally surfaces the tool's own
   `delta.error.error_message`, which is not an `error` packet and would
   otherwise render as nothing at all.

**Security review.** Types, pure functions over arrays, and presentational
components. No network, storage, timer or Tauri call. Model-authored text
reaches the DOM only through Opal's `CompactMarkdown`, which sanitizes with
`rehype-sanitize` against an element allowlist; tool output, file previews and
command strings are rendered as text nodes, never as markup.

**Tests.** `src/conversation/{grouping,findRenderer,renderers}.test.*` — 20
tests, 100% line coverage of the four modules. Each guard was verified by
introducing the regression it claims to catch: eleven mutations, eleven
failures, including a dropped `tab_index` merging two parallel tools, a
reordered dispatch table letting a tool capture the answer, a trailing
`exit_code: null` erasing a real failure, and an `error` packet with no message
rendering as nothing.
