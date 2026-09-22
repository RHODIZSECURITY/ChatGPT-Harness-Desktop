# RHODIZ Harness Desktop — Source Provenance

Every adapted/copied upstream module must be recorded here before merge.
Pins are immutable provenance anchors, never floating build inputs.

| Source | Exact commit | License | Classification | Imported code |
| --- | --- | --- | --- | --- |
| RHODIZSECURITY/openhands | a0403035a20c91decadd011b907ee5b489f6788b | MIT | ADAPT | None yet |
| RHODIZSECURITY/onyx-foss | e11c874019dbf04032cfc3476d82eba1d069a3d8 | MIT | ADOPT/ADAPT | None yet |
| RHODIZSECURITY/librechat | 2452f499a86ae215146d988e9418a481616238ad | MIT | ADAPT | None yet |

## Import rule

For every imported component add: upstream path, exact commit, copyright/license,
local destination, modification summary, security review outcome, and tests.
Backends/agent runtimes from these sources are not execution authorities.
## Adaptations that import no code

Recorded here because the import rule above governs modules, and this is not
one — but leaving it unrecorded would make the table's `None yet` look like a
claim that nothing upstream influenced this tree.

**Semantic token naming, `src/index.css`.** The convention of naming surfaces
by elevation (`surface-deep` / `surface` / `surface-raised` / `surface-overlay`)
rather than by the widget that uses them is taken from openhands' `@theme`
block at the pinned commit. No values and no code were copied: openhands'
tokens resolve through `--heroui-*` HSL channel variables set by the HeroUI
plugin, a dependency this project does not take. The values here are this
project's own, with the two brand colours read off `src-tauri/icons/32x32.png`.

`Imported code` therefore remains `None yet` for all three sources, and that
is exact.
