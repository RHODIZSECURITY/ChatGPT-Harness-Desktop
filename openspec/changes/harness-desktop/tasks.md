# RHODIZ Harness Desktop — lista de ejecución canónica única

Marcado de repositorio destino: **[Core]** = `ChatGPT-Arnes` · **[Desktop]** = `ChatGPT-Harness-Desktop`.

Una tarea **MUST NOT** implementarse en un repositorio distinto al marcado. Una tarea sólo se marca `[x]` con evidencia del SHA vigente; "parece correcto" y "los tests locales pasan" no son evidencia de una obligación de host o de release.

## Continuity checkpoint — 2026-09-20 (America/Chicago)

Estado de partida verificado contra producción, no asumido:

- Core en `main` `53bbc767482b14190a5d91bb58de966f38645864`, servido por el contenedor `534a5ec0813d` en `rhodiz-relay`: 236 herramientas, contrato `rhodiz-arnes` v1, `/events` SSE operativo, doctores en `ok`.
- Desktop en `0.1.0`, rama `feat/windows-desktop-foundation-20260918`, commit `39c402872123f4f582c5ff3d8ea5293756e1a000`: fundación segura con un solo comando IPC (`runtime_status`).
- Evidencia del puente Route → Arnes: 3 corridas PASS con 236 herramientas, identidad OAuth **sintética**.
- Huecos medidos del event stream: sin diffs, sin transiciones de nodo de Graph, sin evento de checkpoint, sin progreso de herramienta, sin eventos LSP, sin semántica de test/lint/build.

Ninguna tarea está completa todavía.

## 0. Bloqueo de plan y anti-deriva

- [x] 0.1 [Core] Establecer este cambio como hoja de ruta canónica del frente Desktop y declarar `HARNESS_DESKTOP_PRODUCT_ROADMAP`.
- [x] 0.2 [Core] Registrar las obligaciones de baseline con evidencia productiva verificada en `baseline-obligations.md`.
- [x] 0.3 [Core] Resolver el conflicto de nombre de distro WSL2 fijando `RHODIZ-Harness` como canónico.
- [x] 0.4 [Core] Actualizar `docs/WINDOWS_DESKTOP_CLIENT_ARCHITECTURE.md`: añadir un puntero hacia adelante que indique que el baseline avanzó desde el ancla `c45295a8…` del `PHASE START`, **sin reescribir ese SHA** porque falsificaría un registro fechado; y unificar el nombre de distro a `RHODIZ-Harness`.
- [x] 0.5 [Core] Corregir la deriva documental real detectada: `README.md` declara `unit: 82/82 PASS` bajo el rótulo "Current local gates" cuando la suite medida sobre ese mismo commit es 136/136. (`README.md` **no** está mal sobre `places_search`: su frase es condicional al operador, y producción lo configuró. `.project/Context.md` es un espejo de continuidad fechado, no documentación de producto, y se deja intacto por la misma razón que el ancla del `PHASE START`.)
- [ ] 0.6 [Desktop] Referenciar este plan desde el repositorio del cliente y prohibir explícitamente una hoja de ruta Desktop paralela.

## 1. Core — completitud del event stream

- [ ] 1.1 [Core] Subir `EVENT_STREAM_SCHEMA_VERSION` a 2 y documentar la regla aditiva: los 15 tipos existentes no se renombran ni cambian de forma, y un `kind` desconocido se ignora en lugar de fallar.
- [ ] 1.2 [Core] Añadir `operation` (`create`/`update`/`delete`/`move`) y tamaños a `file.change`, derivando la operación de la existencia previa del destino y no del nombre de la herramienta; verificar que ningún campo actual desaparece.
- [ ] 1.3 [Core] Implementar el evento `file.diff` con diff unificado acotado (límite por defecto 64 KiB), marca `truncated`, manejo de binarios sin cuerpo, redacción previa a la emisión, y resultado `unavailable` con motivo cuando exceda presupuesto; verificar que nunca bloquea ni retrasa la herramienta que lo originó.
- [ ] 1.4 [Core] Implementar `agent.node` con `agent_id` padre, `node_id`, `status` y temporización; excluir contenido de inferencia; marcar `source: 'derived'` cuando el backend no reporte transiciones por nodo y no inventar estados intermedios.
- [ ] 1.5 [Core] Implementar `continuity.checkpoint` en `checkpoint_context` y `session_checkpoint_context`, con identificadores suficientes para recuperar el checkpoint completo sin incrustar su cuerpo.
- [ ] 1.6 [Core] Implementar `tool.progress` alimentado por el `onprogress` real de conectores MCP y procesos de larga duración; verificar que no se sintetiza progreso artificial.
- [ ] 1.7 [Core] Implementar `lsp.activity` en `lsp-manager.mjs` para diagnósticos, símbolos, definición, referencias y planificación de rename; verificar que el rename sigue sin aplicar ediciones.
- [ ] 1.8 [Core] Implementar `task.result` con clasificación de test/lint/build sobre procesos, sin convertir en resultado semántico un proceso que no lo es.
- [ ] 1.9 [Core] Implementar la política de contrapresión: diffs y progreso son descartables antes que `tool.*`, `process.exit` y `stream.*`; el descarte se señaliza y un consumidor puede distinguir "no hubo diff" de "diff descartado".
- [ ] 1.10 [Core] Verificar que ningún evento nuevo filtra bearer, credencial de provider, secreto de entorno hijo ni contenido de `/run/secrets`, incluyendo el caso de un diff sobre un fichero que contiene un secreto.
- [ ] 1.11 [Core] Extender `tests/mcp-smoke.mjs` y los unitarios para cubrir cada tipo nuevo, el orden monótono de secuencia, el replay y la compatibilidad del consumidor del conjunto anterior.

## 2. Core — superficie de contrato para el Desktop

- [ ] 2.1 [Core] Verificar y documentar que `harness_contract_v1` expone todo lo que el broker necesita para decidir habilitación de UI, y añadir lo que falte sin romper `contract_version` 1.
- [ ] 2.2 [Core] Confirmar con test que el perfil `coding` concede exactamente el conjunto de capacidades que una sesión de programación Desktop necesita, y ni una más.
- [ ] 2.3 [Core] Confirmar que `arnes_doctor_v1` y `provider_doctor_v1` devuelven comprobaciones estructuradas suficientes para la superficie Diagnostics, incluida la distinción entre fallo y omisión.
- [ ] 2.4 [Core] Documentar en `docs/REALTIME_EVENT_STREAM.md` el esquema v2 completo con todos los tipos, campos, política de descarte y reglas de reconexión.

## 3. Core — integridad de release

- [ ] 3.1 [Core] Verificar que `config/release-manifest.schema.json` cubre los digests exigidos: Core, Route, Memory MCP, sidecars habilitados, versión de esquema de Compose/config, versión de migración y mínimos de WSL/Desktop.
- [ ] 3.2 [Core] Especificar y probar `release_sequence` monótono como única autoridad anti-rollback, con test que demuestre que SemVer no actúa como guarda de degradación.
- [ ] 3.3 [Core] Especificar el formato de firma separada `release.json.sig` y publicar vectores de prueba que permitan al broker Rust verificar bytes exactos antes de parsear.
- [ ] 3.4 [Core] Añadir una puerta que rechace etiquetas `:latest` en cualquier manifiesto candidato a certificación.

## 4. Desktop — broker Rust y ciclo de vida WSL2

- [ ] 4.1 [Desktop] Implementar comandos tipados de estado y aprovisionamiento WSL cubriendo las rutas WSL ausente, presente y desactualizado, con resultados fail-closed y diagnóstico accionable.
- [ ] 4.2 [Desktop] Aprovisionar/importar la distro `RHODIZ-Harness` con systemd habilitado y verificado.
- [ ] 4.3 [Desktop] Instalar y configurar Docker Engine y plugin Compose dentro de la distro; verificar que no se publica ningún puerto de Route, Memory ni sidecars hacia Windows.
- [ ] 4.4 [Desktop] Materializar el bundle de runtime versionado con digests fijados y arrancarlo mediante servicio systemd de bootstrap.
- [ ] 4.5 [Desktop] Implementar verify, repair, start, stop y logs como comandos tipados; verificar que no existe ruta que exponga terminal WSL arbitraria por IPC de Tauri.
- [ ] 4.6 [Desktop] Implementar recuperación ante reinicio de Windows y ante fallo del demonio Docker.
- [ ] 4.7 [Desktop] Implementar el almacén de secretos respaldado por el SO o fichero root dentro de WSL; verificar por test que el renderer no puede leerlos y que no aparecen en logs.

## 5. Desktop — negociación de contrato y modelo de sesión

- [ ] 5.1 [Desktop] Negociar `harness_contract_v1` antes de habilitar UI dependiente de capacidades; exigir protocolo `rhodiz-arnes` y `contract_version` 1; verificar que el sondeo de nombres de herramientas no se acepta como sustituto.
- [ ] 5.2 [Desktop] Abrir sesiones de programación con `capability_profile: "coding"` y conservar el lease V2 sólo durante la tarea activa.
- [ ] 5.3 [Desktop] Implementar rotación y expiración de lease, incluida la revocación inmediata de generaciones anteriores; verificar que un lease rotado deja de funcionar.
- [ ] 5.4 [Desktop] Verificar que el lease no se persiste en `localStorage`, `IndexedDB`, parámetros de URL, logs ni ficheros de proyecto.
- [ ] 5.5 [Desktop] Consumir el stream `/events` desde el broker con bearer, reconexión por `runtime_id` + secuencia, y reconstrucción de estado durable desde las APIs de Project/Session/checkpoint tras `stream.reset`.

## 6. Desktop — superficie de actividad en vivo

- [ ] 6.1 [Desktop] Implementar las tres vistas sincronizadas: conversación/progreso, línea temporal de actividad, y paneles de terminal/diff/test.
- [ ] 6.2 [Desktop] Renderizar salida incremental de terminal en continuo, con stdin/PTY bidireccional y cancelación.
- [ ] 6.3 [Desktop] Renderizar diffs en vivo y finales a partir de `file.diff`, distinguiendo visualmente diff ausente de diff descartado por presión.
- [ ] 6.4 [Desktop] Renderizar estado de AgentManager y transiciones de nodo de Graph desde `agent.state` y `agent.node`.
- [ ] 6.5 [Desktop] Renderizar checkpoints, estado de reconexión/replay y errores terminales.
- [ ] 6.6 [Desktop] Implementar solicitudes y decisiones de aprobación **del broker** para operaciones de ciclo de vida del host; verificar que no se presentan como aprobaciones del Core.
- [ ] 6.7 [Desktop] Verificar que el stream renderizado nunca muestra bearers, credenciales de provider ni secretos de entorno hijo.

## 7. Desktop — superficies de programación

- [ ] 7.1 [Desktop] Integrar Projects y Sessions primero, incluido el uso de worktrees aislados para tareas concurrentes.
- [ ] 7.2 [Desktop] Integrar ficheros, editor y terminal sobre operaciones MCP con lease, sin traducir acciones de usuario a comandos de shell crudos cuando exista una operación equivalente.
- [ ] 7.3 [Desktop] Integrar chat, línea temporal de herramientas, estado de plan/goal y artefactos.
- [ ] 7.4 [Desktop] Implementar import/export explícito entre carpetas Windows y almacenamiento de Project gestionado; verificar que no existe workspace compartido escribible silencioso.
- [ ] 7.5 [Desktop] Registrar en `PROVENANCE.md` cada componente importado de los tres pins MIT con ruta, commit, licencia, modificación, revisión de seguridad y cobertura.

## 8. Desktop — updater y rollback

- [ ] 8.1 [Desktop] Verificar la firma separada sobre los bytes exactos del manifiesto **antes** de parsear cualquier URL, versión o digest.
- [ ] 8.2 [Desktop] Implementar el flujo transaccional: verificar manifiesto → verificar digests → preparar N+1 → migrar → arrancar → negociar contrato → doctor y smoke → activar atómicamente.
- [ ] 8.3 [Desktop] Implementar rollback acotado al release N y demostrarlo tras un candidato deliberadamente malo.
- [ ] 8.4 [Desktop] Verificar que `release_sequence` impide degradación y que SemVer no actúa como guarda.
- [ ] 8.5 [Desktop] Mantener el updater ejecutable de Tauri como eje independiente del updater del runtime WSL/Docker.

## 9. Certificación de seguridad

- [ ] 9.1 [Desktop] Demostrar que el renderer no puede invocar comandos arbitrarios de WSL, PowerShell ni Docker.
- [ ] 9.2 [Desktop] Demostrar que el renderer no puede leer credenciales de backend almacenadas.
- [ ] 9.3 [Core+Desktop] Regresión de aislamiento Project/Session desde el camino Desktop: sin autoridad cruzada de ruta ni de proceso entre proyectos o sesiones.
- [ ] 9.4 [Core+Desktop] Regresión de Landlock y Resource Governor desde el camino Desktop.
- [ ] 9.5 [Core+Desktop] Verificar que el Desktop no monta `docker.sock` en el harness y que Route/Memory sanos no conceden autoridad de workspace por sí mismos.
- [ ] 9.6 [Desktop] Verificar CSP estricta, DevTools desactivado en producción y navegación anclada a orígenes empaquetados.

## 10. Certificación Windows

Ninguna tarea de esta sección puede completarse con evidencia obtenida en Linux.

- [ ] 10.1 [Desktop] Instalación en máquina limpia Windows 11.
- [ ] 10.2 [Desktop] Rutas WSL ausente, WSL presente y WSL desactualizado.
- [ ] 10.3 [Desktop] Reinicio con recuperación de backend.
- [ ] 10.4 [Desktop] Fallo del demonio Docker y reparación.
- [ ] 10.5 [Desktop] Verificación de imágenes fijadas por digest exacto.
- [ ] 10.6 [Desktop] Exposición sólo en loopback bajo red normal y bajo VPN.
- [ ] 10.7 [Desktop] Rollback de actualización tras candidato malo.
- [ ] 10.8 [Desktop] Desinstalación que preserva o exporta explícitamente los datos de Project.
- [ ] 10.9 [Desktop] Instalador, firma de código y E2E Windows completo.

## 11. Aceptación final

- [ ] 11.1 [Core] Puertas locales en verde sobre el SHA candidato: `npm run check`, `npm test`, `npm run test:mcp`, `docker compose config --quiet`, `docker compose build`. Instalar dependencias en la raíz y en cada sub-paquete de `providers/` antes de medir; su ausencia produce fallos de entorno que MUST NOT contarse como regresión.
- [ ] 11.2 [Desktop] Puertas en verde sobre el SHA candidato: `verify:portable` y `verify:windows`.
- [ ] 11.3 [Core+Desktop] CI de SHA exacto en verde en ambos repositorios.
- [ ] 11.4 [Core+Desktop] Revisión de dependencias, licencias y SBOM, más inspección de avisos de seguridad.
- [ ] 11.5 [Core+Desktop] Verificar que ninguna obligación de `baseline-obligations.md` quedó debilitada.
- [ ] 11.6 [Core+Desktop] Confirmar que este cambio no duplicó ni contradijo el plan `remote-access` de Route.
- [ ] 11.7 Autorización explícita del operador para el release.
