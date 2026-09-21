# RHODIZ Harness Desktop — canonical product change

Este cambio OpenSpec es la **hoja de ruta y contrato de gobernanza única del frente RHODIZ Harness Desktop**. Todo el alcance de producto, las obligaciones de seguridad, la arquitectura, las tareas ejecutables y las especificaciones activas viven bajo `openspec/changes/harness-desktop/`.

`HARNESS_DESKTOP_PRODUCT_ROADMAP=openspec/changes/harness-desktop/`

Ningún agente puede crear ni seguir una hoja de ruta Desktop competidora. `docs/WINDOWS_DESKTOP_CLIENT_ARCHITECTURE.md` sigue siendo la fuente arquitectónica narrativa; este cambio la convierte en obligaciones verificables y, donde ambos se contradigan, **gana el requisito más estricto**.

## Relación con RemoteAccess

RemoteAccess tiene su propia hoja de ruta canónica en `RHODIZSECURITY/RHODIZ-MCP-Route`, bajo `openspec/changes/remote-access/`. Este cambio **MUST NOT** duplicarla, bifurcarla ni contradecirla.

La frontera es explícita:

- El plan `remote-access` gobierna identidad remota, OAuth, Relay, pairing, enrolamiento de dispositivos, grants, cuotas y el perfil de puente MCP downstream del Client.
- Este plan gobierna el **cliente Desktop de Windows** y el **contrato que el Core le debe**.
- Cloud ChatGPT Arnes aparece en el plan `remote-access` como *downstream MCP service profile* — un adaptador dentro del Client de Route, no una cuarta parte de producto. Esa clasificación se respeta aquí sin cambios.

Un despliegue Desktop sano **MUST NOT** presentarse como certificación de RemoteAccess, ni al revés.

## Objetivo de producto

Entregar un cliente de escritorio Windows de grado comercial para el runtime RHODIZ Harness, que dé al usuario una superficie de programación con continuidad y evidencia en vivo, **sin convertirse en una segunda autoridad de ejecución**.

El Harness Core certificado sigue siendo la autoridad única sobre aislamiento Project/Session, leases de workspace firmados, aplicación de capacidades, confinamiento Landlock y gobierno de recursos. El Desktop presenta, no decide.

Topología canónica:

`Usuario Windows -> shell Tauri 2 -> broker Rust acotado -> distro WSL2 RHODIZ-Harness -> Docker/Compose -> Harness Core + RHODIZ MCP Route + RHODIZ Memory MCP + sidecars de provider -> Project / Session / lease firmado`

## Partes del entregable

El frente tiene dos repositorios y una frontera nítida entre ellos:

1. **Harness Core** — `RHODIZSECURITY/ChatGPT-Arnes`. Debe publicar y sostener el contrato que el Desktop consume: negociación `harness_contract_v1`, proyección de capacidades con perfil `coding`, superficies doctor, event stream operativo completo y manifiestos de release firmados.
2. **Harness Desktop** — `RHODIZSECURITY/ChatGPT-Harness-Desktop`. Shell Tauri 2, renderer React/TypeScript, broker Rust acotado, ciclo de vida WSL2/Docker, updater transaccional con rollback y las puertas de certificación Windows.

Las tareas de este plan llevan marcado su repositorio destino. Un mismo plan gobierna ambos porque el contrato sólo tiene sentido verificado de extremo a extremo.

## Punto de partida real

Estado verificado el 2026-09-20, no asumido:

- **Core** en producción (`rhodiz-relay`, contenedor `534a5ec0813d`) sirviendo `main` en `53bbc767482b14190a5d91bb58de966f38645864`: 236 herramientas MCP, contrato `rhodiz-arnes` v1, proyección de capacidades con cuatro perfiles, `/events` SSE con `runtime_id` + secuencia + replay, y `arnes_doctor_v1` / `provider_doctor_v1` en `ok`.
- **Desktop** en `0.1.0`, rama `feat/windows-desktop-foundation-20260918`: fundación segura con renderer local, CSP estricta, DevTools desactivado en producción, **un solo comando IPC** (`runtime_status`), sonda fija `wsl.exe --status`, sin plugin genérico de shell/proceso, y anclas de procedencia registradas sin código importado todavía.

El frente está, por tanto, en el arranque: la fundación de seguridad existe y el contrato de producto no.

## Huecos que motivan el cambio

Medidos contra `docs/WINDOWS_DESKTOP_CLIENT_ARCHITECTURE.md`, que declara el stream de actividad en vivo **requisito de producto, no telemetría opcional**:

- El Core emite hoy 15 tipos de evento (`tool.*`, `process.*`, `file.change`, `git.change`, `agent.state`, `stream.*`). Los deltas incrementales de stdout/stderr, la cancelación y el estado de salida ya funcionan.
- **Falta**: diffs y tipo de operación en `file.change`; transiciones de nodo del Graph; evento propio de checkpoint; progreso de herramienta MCP; semántica de test/lint/build; solicitudes y decisiones de aprobación; y actividad LSP, que no emite ningún evento.
- El Desktop no negocia todavía `harness_contract_v1` ni abre sesiones con perfil `coding`.
- No existe manifiesto de release firmado operativo ni flujo de rollback verificado.
- El nombre de la distro WSL2 está en conflicto entre documentos: `RHODIZ-Arnes` en el doc de arquitectura frente a `RHODIZ-Harness` en el cliente. Este plan fija **`RHODIZ-Harness`** como canónico.

## Invariantes de producto

- El Desktop **MUST NOT** convertirse en una segunda autoridad de programación ni reimplementar el Core.
- El renderer **MUST NOT** recibir autoridad genérica de PowerShell, `wsl.exe`, Docker, filesystem o proceso; invoca sólo un allowlist tipado de comandos del broker Rust.
- Ningún bearer del Harness ni credencial de backend **MUST** alcanzar el renderer, `localStorage`, `IndexedDB`, parámetros de URL ni logs.
- El Desktop **MUST NOT** montar `docker.sock` en el Harness.
- La cadena `tenant -> principal -> Project -> Session -> lease firmado` se preserva sin excepción desde el camino Desktop.
- Route y Memory MCP son servicios de primera parte del runtime gestionado, nunca autoridades de ejecución alternativas. Un Route o Memory sanos no conceden autoridad de workspace por sí mismos.
- Etiquetas `:latest` **MUST NOT** aceptarse en releases certificadas; toda actualización de backend exige manifiesto firmado, digests exactos y secuencia anti-rollback monótona.
- Todo lo que falle — contrato ausente, credencial inválida, digest no verificado, provider sin configurar — **SHALL** fallar cerrado.

## Alcance requerido para 10/10 comercial

- Instalación en máquina limpia Windows 11, con las rutas WSL ausente, presente y desactualizado.
- Ciclo de vida completo del runtime gestionado: install, verify, repair, start, stop, logs, upgrade y rollback, sin exponer terminal WSL arbitraria por IPC de Tauri.
- Negociación de contrato Core antes de habilitar UI dependiente de capacidades.
- Stream de actividad en vivo con las tres vistas sincronizadas: conversación/progreso, línea temporal de actividad, y panel de terminal/diff/test.
- Superficies Projects/Sessions, ficheros/editor/terminal, chat/timeline y artefactos.
- Instalador, firma de código, updater y rollback verificados tras un candidato deliberadamente malo.
- E2E Windows con reinicio, reconexión, fallo del demonio Docker, red/VPN y recuperación ante fallo.
- Desinstalación que preserve o exporte explícitamente los datos de Project del usuario.

## Non-goals

- Sustituir o reimplementar RHODIZ MCP Route.
- Reimplementar el Cloud/Core Arnes en Rust o Windows.
- Empotrar simultáneamente los backends de OpenHands, Onyx y LibreChat.
- Habilitar por defecto credenciales externas de LLM, búsqueda, imagen o lugares.
- Exposición LAN pública.
- Ejecución directa contra rutas arbitrarias del host Windows.
- Reclamar paridad con internals propietarios de escritorio.

## Regla de finalización

El frente Harness Desktop no está `done`, `10/10`, `production complete` ni `commercial-ready` hasta que: las dos partes funcionen juntas sobre el mismo candidato certificado; todas las tareas aplicables de este cambio estén completas con evidencia del SHA vigente; las puertas locales y el CI de SHA exacto estén en verde en ambos repositorios; la certificación Windows se haya ejecutado en un entorno Windows real — una comprobación cruzada en Linux **MUST NOT** tratarse como evidencia equivalente —; y el operador autorice explícitamente el release.
