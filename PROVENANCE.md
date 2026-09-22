# RHODIZ Harness Desktop — Source Provenance

Every adapted/copied upstream module must be recorded here before merge.
Pins are immutable provenance anchors, never floating build inputs.

| Source | Exact commit | License | Classification | Imported code |
| --- | --- | --- | --- | --- |
| RHODIZSECURITY/openhands | a0403035a20c91decadd011b907ee5b489f6788b | MIT | ADAPT | None yet |
| RHODIZSECURITY/onyx-foss | e11c874019dbf04032cfc3476d82eba1d069a3d8 | MIT, **except `web/src/ee/` and `backend/ee/`** (see below) | ADOPT/ADAPT | `web/lib/shared/tokens/*.json` → `src/design/tokens/` |
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
