# Nota de copia — origen, autorización y reconciliación

> Este fichero es específico de la copia en el repositorio **Desktop**
> (`ChatGPT-Harness-Desktop`). No existe en la copia canónica. Registra por qué
> hay dos copias del mismo plan y qué las diferencia hoy. Léelo antes de tratar
> cualquiera de las dos como única verdad.

## Qué es esta carpeta

Copia **verbatim** del plan OpenSpec canónico del frente RHODIZ Harness Desktop.
El árbol `proposal.md` · `design.md` · `tasks.md` · `baseline-obligations.md` ·
`specs/harness-desktop/**` es idéntico byte a byte a su origen en la fecha de
copia (verificado con `diff -rq`, resultado IDÉNTICO).

## De dónde viene

| | |
| --- | --- |
| Repositorio origen | `RHODIZSECURITY/ChatGPT-Arnes` (Harness Core) |
| Rama origen | `feat/openspec-harness-desktop-20260920` (en sincronía con `origin`) |
| Commits origen | `f6ddd97` (crea el plan) · `cdb2266` (corrige deriva documental) |
| Ruta origen | `openspec/changes/harness-desktop/` |
| Fecha de copia | 2026-09-20 (America/Chicago) |

## Autorización del override (obligatoria para que esta copia exista)

El propio plan **prohíbe** una hoja de ruta Desktop paralela:

- `proposal.md`, línea 7: *"Ningún agente puede crear ni seguir una hoja de ruta
  Desktop competidora."*
- `tasks.md`, tarea 0.6: *"Referenciar este plan desde el repositorio del cliente
  y prohibir explícitamente una hoja de ruta Desktop paralela."*
- `baseline-obligations.md`: un requisito que entre en conflicto con el baseline
  solo procede *"salvo que el operador autorice explícitamente el cambio y este
  documento se actualice en el mismo commit."*

El operador autorizó **explícitamente** copiar el plan completo a este repositorio
el 2026-09-20, eligiendo esa opción por encima de "referencia canónica" y de
"mover la propiedad". Esta nota es el registro de esa autorización, escrito en el
mismo commit que introduce la copia, como exige el candado anti-regresión.

## Consecuencia asumida: dos copias divergen

El motivo por el que el plan prohibía duplicar es real y sigue en pie: **dos
copias del mismo roadmap se desincronizan a la primera edición**. Mientras existan
ambas:

- Un cambio en el alcance, en una tarea o en una obligación **MUST** aplicarse a
  las dos copias, o quedará una mintiendo.
- Ante cualquier contradicción entre esta copia y la de `ChatGPT-Arnes`, **gana el
  requisito más estricto** hasta que el operador decida cuál es la fuente única.
- La copia canónica de `ChatGPT-Arnes` sigue gobernando las tareas marcadas
  `[Core]`; esta copia es la de trabajo para las tareas `[Desktop]`.

## Reconciliación del checkpoint (el plan quedó por detrás de este repo)

`tasks.md` fija su punto de partida en el Desktop a `0.1.0`, rama
`feat/windows-desktop-foundation-20260918`, con **un solo comando IPC**
(`runtime_status`). **Ese registro está fechado y NO se reescribe aquí** — hacerlo
falsificaría un checkpoint, el mismo criterio que el plan aplica al ancla del
documento de arquitectura.

Lo que sí se anota, medido en este repo el 2026-09-20: el Desktop **ya avanzó más
allá** de ese punto. En la rama `feat/windows-wsl-lifecycle-20260920` (commit
`61d3ce2`) existen **cinco comandos IPC** —`runtime_status`, `runtime_provision`,
`runtime_start`, `runtime_stop`, `runtime_logs`— y la exclusión de ciclo de vida
entre procesos (mutex en proceso + lock de fichero bajo `%LOCALAPPDATA%`).

Impacto en las casillas de la sección 4, que siguen sin marcar porque exigen
evidencia de host/Windows que aún no existe:

| Tarea | Estado real medido | Qué falta |
| --- | --- | --- |
| 4.1 estado + aprovisionamiento WSL tipados, fail-closed | **Parcial** | `runtime_status` clasifica ready/stopped/missing/unavailable; `runtime_provision` **diagnostica antes de negarse** y cubre las cuatro rutas (WSL ausente / insondeable / presente pero por debajo del mínimo fijado / suficiente, donde defiere a la puerta del manifiesto), cerrado en `feat/wsl-provision-preflight-20260920`, commits `2945b41`+`9ee8a08`, PR #5. Sigue **sin crear, descargar ni instalar nada en ninguna rama**: la puerta del manifiesto firmado no existe todavía. La sonda de versión es la única lectura de stdout del broker fuera de los logs (`wsl.exe --version` emite UTF-16LE) y es fail-closed en cada paso: captura truncada se descarta en vez de parsearse, y una versión indeterminable es negativa, nunca un pase por supuesto. Falta la certificación Windows: la **forma real en bytes** de la salida de `wsl.exe --version` nunca se ha observado — el decodificador y el escaneo solo han visto bytes sintetizados |
| 4.5 verify, repair, start, stop, logs tipados | **Parcial** | los **cinco comandos existen y están acotados** (verify y repair cerrados en `feat/runtime-verify-repair-20260920`, commit `e1bf7e8`, PR #4); falta la certificación Windows: exit codes reales de `systemctl is-active` a través de `wsl.exe` (el supuesto del `3` queda escrito como supuesto), el comportamiento real de `repair` contra una unidad fallida, y la contención del lock entre procesos |
| 4.2 distro `RHODIZ-Harness` con systemd | Pendiente | — |
| 4.3 Docker + Compose dentro de la distro | Pendiente | — |
| 4.4 bundle de runtime con digests fijados | Pendiente | — |
| 4.6 recuperación ante reinicio / fallo de Docker | Pendiente | — |
| 4.7 almacén de secretos | Pendiente | — |

Actualización de la fila 4.5, medida en este repo el 2026-09-20: en la rama
`feat/runtime-verify-repair-20260920` (commit `e1bf7e8`, PR #4 hacia
`feat/windows-wsl-lifecycle-20260920`) ya existen **siete comandos IPC** —los
cinco anteriores más `runtime_verify` y `runtime_repair`— con el diseño cerrado
del plan: verify es solo lectura y **no toma el lock** (instantánea; puede
observar estados transitorios durante una mutación), repair toma el lock para sus
dos spawns (`reset-failed` best-effort, `restart` decide), y el reinicio a nivel
de distro **se omite a propósito** (decisión del operador del mismo día). La
superficie de siete comandos está fijada por los contratos anti-crecimiento
(`bridge.test.ts`, `tests/security-contract.test.ts`). La casilla sigue **Parcial**
porque su cierre exige lo que la sección 10 reserva al host Windows, no por
código pendiente.

El lock entre procesos está **type-chequeado, no ejercitado en Windows** (ver
`docs/SECURITY_BASELINE.md`): dos procesos reales disputándoselo no se prueba hasta
correr en un host Windows, y eso pertenece a la sección 10 (certificación Windows),
que no admite evidencia obtenida en Linux.

## Sobre `config.yaml`

`openspec/config.yaml` se copió verbatim y su bloque `context` describe el repo
**origen** (`RHODIZSECURITY/ChatGPT-Arnes`) y sus reglas de autoridad. Esas reglas
siguen siendo ciertas sobre el frente completo; no se reescriben aquí para no
falsear la copia. Si esta copia pasa a ser la fuente única, ese bloque es lo
primero que hay que actualizar.

## Tarea 0.6 cerrada en el Desktop (2026-09-20)

Tarea 0.6 *"Referenciar este plan desde el repositorio del cliente y prohibir
explícitamente una hoja de ruta Desktop paralela"* está cerrada en este repo:
`README.md` referencia `openspec/changes/harness-desktop/` como único plan de
ejecución, declara que el progreso se anota solo en esta nota, que `tasks.md`
permanece byte-idéntico a la copia canónica de Core y que una hoja de ruta
Desktop paralela queda prohibida. Commit `aeafbd7` en la rama
`docs/openspec-harness-desktop-plan-20260920` (PR #3). La casilla en `tasks.md`
no se marca aquí por diseño: esa copia permanece byte-idéntica a la canónica.
