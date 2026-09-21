# Reconocimiento de las fuentes de interfaz

`PROVENANCE.md` fija tres repositorios de referencia con sus commits, y
`.project/ProjectMemory.md` declara qué superficie aporta cada uno y qué
explícitamente no. Hasta ahora esas dos afirmaciones no se habían contrastado
contra el código: eran decisiones registradas, no comprobadas.

Este documento registra lo que se observó al leer los árboles, y en particular
dónde lo observado **corrige** lo que se creía.

## Procedencia de lo leído

Los árboles viven en `/home/claude-rhodiz/workspaces/reference/<repo>`,
deliberadamente fuera de este checkout para que nada pueda arrastrarlos a un
commit. Antes de leer una línea se verificó `git rev-parse HEAD` contra la
tabla de `PROVENANCE.md`: los tres coinciden exactamente. Una tabla de
procedencia no vale nada si se confirma preguntándole a quien colocó los
ficheros.

## openhands — el hallazgo que cambia el análisis

El README describe "Agent Canvas" como un centro de control construido en
torno a **backends de agente enchufables**, que es justo lo que la regla de
`ProjectMemory.md` excluye. Leído sólo eso, la conclusión razonable es que la
clasificación `ADAPT` no sobrevive: si la conmutación de backend es una
función de primera clase de la interfaz, "toma la UI y deja el runtime" no es
un problema de elegir carpetas.

El árbol dice otra cosa, y es más preciso:

- **29 de 633 componentes `.tsx`** referencian el concepto de backend. Un 4,6%,
  no un tejido que lo atraviese todo.
- **Existe una costura real.** `src/api/backend-registry/` y
  `src/api/agent-server-adapter.ts` centralizan la noción; los componentes
  entran por un contexto (`useActiveBackend()`) y un hook
  (`useBackendScopedPath()`), no por llamadas dispersas.
- **La UI de backends es su propia carpeta**: `components/features/backends/`
  (selector, modal de alta, estado, versión). Cuatro de los 29.

Pero hay un matiz que sí importa, y no se ve desde el README ni desde el
recuento: en `conversation-panel.tsx` la identidad del backend está metida en
el **modelo de estado**, no sólo en la presentación —
`pinsByBackendId[activeBackend.id]`, `archivesByBackendId[...]`, paginación
por backend, y ramas sobre `activeBackend.kind !== "local"`.

### Conclusión

`ADAPT` sobrevive, y la regla negativa de `ProjectMemory.md` es aplicable
porque hay una costura definible: prescindir de `features/backends/` y
`api/backend-registry/`, y colapsar `useActiveBackend()` a un runtime local
fijo. No es gratis —hay que deshacer el particionado de estado por backend en
unos cinco ficheros del panel de conversación— pero es acotado y localizable,
que es precisamente lo que "no separable" negaba.

Dicho de otro modo: la impresión alarmante venía de un documento de
presentación, y el código la desmiente. Se registra aquí para que nadie
vuelva a derivarla del README.

## onyx-foss — confirmado como fuente de Tauri

`ProjectMemory.md` lo declara fuente primaria de Tauri/ventana/build. Se
sostiene: `desktop/` es un workspace con `src-tauri/tauri.conf.json` y su
propio `Cargo.toml`, y usa **las mismas versiones que este repositorio**
(`@tauri-apps/api` 2.11.1, `@tauri-apps/cli` 2.11.4). No es una referencia de
una generación distinta de Tauri, que es el riesgo que haría inservible el
código de ventana y build.

## librechat — todavía sin reconocer

No se ha leído. La clasificación `ADAPT` sigue siendo una decisión registrada
y no comprobada, igual que lo eran las otras dos antes de este documento. No
se debe citar como verificada.

## Qué NO establece esto

Nada de aquí es una importación. Las tres filas de `PROVENANCE.md` siguen
diciendo `Imported code: None yet`, y eso es exacto. Cuando exista la primera
importación, la regla de `PROVENANCE.md` sigue vigente: ruta upstream, commit,
licencia, destino, resumen de modificación, revisión de seguridad y tests.

Tampoco es una revisión de seguridad de esos árboles. Es un reconocimiento de
estructura para decidir qué se puede tomar y por dónde corta.
