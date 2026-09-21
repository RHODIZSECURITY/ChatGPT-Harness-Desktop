# Obligaciones de baseline — candado anti-regresión

Este documento fija lo que el Harness Core **ya tiene certificado** antes de que empiece el frente Desktop. Toda obligación listada aquí es **vinculante**: el trabajo del Desktop puede añadir sobre ella, y **MUST NOT** debilitarla, eludirla ni reinterpretarla como opcional.

Donde una obligación de baseline y un requisito nuevo entren en conflicto, **gana la obligación de baseline** salvo que el operador autorice explícitamente el cambio y este documento se actualice en el mismo commit.

## Ancla de evidencia

Verificado el 2026-09-20 contra el despliegue productivo real en `rhodiz-relay`, no contra documentación:

| Elemento | Valor verificado |
| --- | --- |
| Rama / SHA de producción | `main` · `53bbc767482b14190a5d91bb58de966f38645864` |
| Checkout productivo | `/opt/rhodiz-harness/source`, árbol limpio, en sincronía con `origin/main` |
| Contenedor | `534a5ec0813d` · imagen `sha256:be0282d39183…` · `healthy` · `restarts=0` |
| `build_sha` servido | `53bbc767482b14190a5d91bb58de966f38645864` |
| Catálogo MCP | 236 herramientas en perfil `full` · 1 prompt canónico |
| Contrato | protocolo `rhodiz-arnes` · `contract_version` 1 · `workspace_lease` v2 |

**Nota sobre el pin del documento de arquitectura**: `docs/WINDOWS_DESKTOP_CLIENT_ARCHITECTURE.md` ancla el baseline en `c45295a81f4f3341c902842bb5d65c1f7180a41b` (`Merge PR #7: certify cloud core baseline`, 2026-09-17) bajo un encabezado `PHASE START — 2026-09-18`. Ese ancla **era exacta en esa fecha** y MUST NOT sustituirse: reescribir el SHA falsificaría un registro fechado. Desde entonces el baseline avanzó hasta el SHA de la tabla anterior. La tarea 0.4 añade un puntero hacia adelante, no una corrección del ancla histórica.

## B1 — Autoridad de ejecución

El Harness Core es la **única** autoridad sobre efectos de workspace. El Desktop, su broker Rust, su renderer, RHODIZ MCP Route y RHODIZ Memory MCP **MUST NOT** ejecutar efectos de Project/Session por su cuenta.

- La cadena `tenant -> principal -> Project -> Session -> workspace_lease firmado` **SHALL** preservarse sin excepción.
- Sólo un lease firmado emitido por el servidor autoriza herramientas `session_*`. Un `project_id`, `session_id` o ruta elegidos por el cliente **MUST NOT** constituir autoridad.
- La rotación de lease (`workspace_session_resume`) **SHALL** revocar de inmediato las generaciones anteriores. Verificado: un lease rotado falla con `rotated or revoked`.
- Un lease manipulado **SHALL** fallar cerrado. Verificado.
- Los manifiestos de capacidad firmados por sesión **SHALL** aplicarse centralmente en cada llamada `session_*`. Verificado: un lease `diagnostics` permite lectura Git y deniega `session_write_file`.

## B2 — Frontera Docker

La composición productiva verificada **SHALL** mantenerse como mínimo:

- usuario no root (`node`);
- `no-new-privileges:true`;
- `CapDrop=[ALL]` y `CapAdd=[]`;
- sistema de ficheros raíz de sólo lectura;
- montajes de escritura limitados a `/workspace`, `/projects` y `/state`;
- secretos montados de sólo lectura bajo `/run/secrets`;
- **sin** `docker.sock` montado en el harness;
- puerto MCP publicado sólo en `127.0.0.1`.

El Desktop **MUST NOT** relajar ninguno de estos puntos para simplificar su instalador.

## B3 — Autenticación y secretos

- `/mcp`, `/info` y `/events` **SHALL** exigir bearer. Verificado en vivo: 401 sin credencial; `/healthz` responde 200 y es el único endpoint no autenticado.
- El bearer del harness, el token de Memory y las credenciales de provider **MUST NOT** heredarse a procesos hijo ni aparecer en logs, notas, historial, memoria, fixtures, commits ni salida visible al usuario.
- El renderer del Desktop **MUST NOT** recibir ni persistir ninguna de esas credenciales.

## B4 — Gobierno de recursos

- El techo duro es cgroup de Docker. Verificado: `cgroup-v2`, 8192 MB, 4000 millicpu, 512 PIDs.
- El Resource Governor **SHALL** seguir haciendo control de admisión por sesión dentro de ese techo.
- `per_session_cgroup` es hoy `false` y los bytes de worktree son **cuota blanda**. Ninguna superficie Desktop **MUST** presentarlos como aislamiento duro de disco por sesión.

## B5 — Confinamiento de procesos

El confinamiento Landlock fail-closed de subprocesos en producción **SHALL** mantenerse: una shell de sesión no puede abrir `/run/secrets`, `/state`, `/workspace` ni el worktree de otra sesión por ruta absoluta.

## B6 — Capacidades fail-closed

- `web`, `web_search` e `image_gen` **SHALL** seguir fallando cerrado mientras no haya backend aprobado configurado. Verificado en producción: `backend_available: false`.
- Un provider no configurado **MUST NOT** fabricar resultados ni sustituirse por otro backend.
- El proveedor de agentes (`strands-agent-provider`) está hoy sin configurar. El Desktop **MUST NOT** afirmar que hay agentes en segundo plano cuando no hay backend disponible.

## B7 — Contrato y prompt canónico

- `prompts/HARNESS_PROMPT.md` **SHALL** seguir enviándose en `initialize.instructions`, publicándose como prompt MCP `cloud-chatgpt-arnes` y expuesto por `harness_prompt` como respaldo.
- Ningún artefacto de este cambio **MUST** introducir un segundo prompt de comportamiento divergente.
- `harness_contract_v1` **SHALL** seguir siendo la superficie de negociación. Sondear nombres de herramientas o cadenas de versión **MUST NOT** aceptarse como sustituto.

## B8 — Frontera con Route

- Route es transporte y admisión; **MUST NOT** convertirse en un segundo harness de programación ni duplicar implementaciones de herramientas.
- Evidencia vigente del puente (`route-harness-e2e-236.log`, 2026-09-19, tres corridas PASS): 236 herramientas, 1 prompt, `initialize.instructions`, `tools/list`, `prompts/get`, create/write/read/patch/exec/git, cancelación y reconexión de Agent, con `route_workspace_authority: false` y `route_docker_socket_authority: false`.
- **Límite explícito de esa evidencia**: `oauth_identity: "synthetic-test-identity"`. La certificación OAuth con host aprobado sigue abierta y pertenece a la tarea 2.18 del plan `remote-access` de Route, **no a este cambio**.

## B9 — Puertas de verificación

Las puertas locales vigentes **SHALL** permanecer en verde y ninguna tarea Desktop puede declararse completa con ellas rotas:

```text
npm run check          # sintaxis
npm test               # 136/136 unitarios
npm run test:mcp       # MCP_SMOKE_PASS tools=236 contract=1
docker compose config --quiet
docker compose build
```

Medido localmente el 2026-09-20 sobre `53bbc76`, no copiado de documentación: **136/136 unitarios en verde** y `MCP_SMOKE_PASS tools=236 prompt_sha256=e8b8fb28d88a contract=1`.

`README.md` declara "unit: 82/82 PASS"; esa cifra corresponde a un checkpoint anterior y está obsoleta. La suite creció a 136. Corregir el README es tarea 0.5 de este cambio.

La verificación exige dependencias instaladas en la raíz **y** en cada sub-paquete de `providers/`. Sin ellas fallan 9 unitarios por `ERR_MODULE_NOT_FOUND`, lo que es un fallo de entorno y **MUST NOT** interpretarse como regresión.

Cobertura de nombres Astra/Fable: 0 ausentes en perfil `full`. Inventario de toolkit: 37 agentes RHODIZ, 59 skills RHODIZ, 14 skills Codex, 11 descriptores Fable.

## B10 — Event stream ya existente

Los 15 tipos de evento que el Core ya emite **SHALL** conservar su semántica y su orden monótono:

`tool.start` · `tool.finish` · `tool.error` · `process.start` · `process.stdout` · `process.stderr` · `process.error` · `process.exit` · `process.signal` · `file.change` · `git.change` · `agent.state` · `stream.ready` · `stream.reset` · `stream.shutdown`

- La reconexión por `runtime_id` + secuencia y la emisión de `stream.reset` ante cambio de runtime, cursor futuro o ventana de replay excedida **SHALL** mantenerse.
- La redacción consciente de secretos del stream **SHALL** mantenerse.
- Los eventos nuevos que introduzca este cambio **MUST** añadirse sin romper consumidores del conjunto anterior.
