# Diseño — RHODIZ Harness Desktop

## 1. Frontera de repositorios

Un único plan gobierna dos repositorios porque el contrato sólo tiene sentido verificado de extremo a extremo. La frontera de implementación es estricta:

| Responsabilidad | Repositorio |
| --- | --- |
| Event stream, contrato, proyección de capacidades, doctores, esquema de manifiesto de release | `ChatGPT-Arnes` |
| Shell Tauri, renderer, broker Rust, ciclo WSL2/Docker, updater, empaquetado, E2E Windows | `ChatGPT-Harness-Desktop` |
| Identidad remota, OAuth, Relay, pairing, grants | `RHODIZ-MCP-Route` (plan `remote-access`, **fuera de este cambio**) |

Cada tarea de `tasks.md` lleva su repositorio destino entre corchetes. Una tarea **MUST NOT** implementarse en un repositorio distinto al marcado.

## 2. Decisión resuelta: nombre de la distro WSL2

Había conflicto: `RHODIZ-Arnes` en `docs/WINDOWS_DESKTOP_CLIENT_ARCHITECTURE.md` frente a `RHODIZ-Harness` en el README del cliente.

**Decisión: `RHODIZ-Harness`.** Razón: toda la nomenclatura viva del frente ya converge ahí (`ChatGPT-Harness-Desktop`, `@rhodiz/harness-desktop`, `rhodiz-harness-broker-core`), y el cliente es el artefacto más reciente y el que realmente crea la distro. El documento de arquitectura se corrige, no al revés.

El nombre de la distro es **entrada de instalador**, no cadena cosmética: cambiarlo después de que existan distros instaladas exige migración. Se fija ahora, antes de que haya instaladores en circulación.

## 3. Extensión del event stream

### 3.1 Principio

Los 15 tipos actuales son **aditivos e intocables** (baseline B10). Los tipos nuevos se añaden; ninguno se renombra ni cambia de forma. Un consumidor del conjunto anterior debe seguir funcionando sin cambios.

`EVENT_STREAM_SCHEMA_VERSION` sube a `2`. La subida señala *campos añadidos*, nunca campos retirados. El Desktop negocia por contrato, no por versión de esquema: un cliente que no entienda un `kind` desconocido **MUST** ignorarlo en lugar de fallar.

### 3.2 Tipos nuevos

| `kind` | Origen | Propósito |
| --- | --- | --- |
| `file.diff` | `mcp.mjs` tras mutación exitosa | Resumen y diff acotado del efecto sobre ficheros |
| `agent.node` | `agent-manager.mjs` | Transición de un nodo individual del Graph |
| `continuity.checkpoint` | `mcp.mjs` en `checkpoint_context` / `session_checkpoint_context` | Hito de continuidad en la línea temporal |
| `tool.progress` | `mcp.mjs` vía `onprogress` de MCP | Progreso intermedio de herramienta de larga duración |
| `lsp.activity` | `lsp-manager.mjs` | Diagnósticos, símbolos, definición, referencias y planificación de rename |
| `task.result` | `process-manager.mjs` con clasificación | Resultado semántico de test/lint/build |

### 3.3 `file.change` y `file.diff`

`file.change` **se mantiene tal cual** por compatibilidad. Se le añaden campos opcionales; no se le quita ninguno:

- `operation`: `create` | `update` | `delete` | `move`, derivado de la existencia previa del destino, no del nombre de la herramienta.
- `bytes_before` / `bytes_after`.

El diff viaja en un evento **separado**, `file.diff`, por tres razones: un diff puede ser grande y no debe inflar el evento de cambio; puede fallar su generación sin invalidar el hecho del cambio; y permite que un consumidor ligero ignore diffs sin filtrar campos.

Restricciones de `file.diff`:

- diff unificado con límite duro de bytes configurable (por defecto 64 KiB); al excederlo se emite `truncated: true` y sólo el `--stat`.
- ficheros binarios: sin cuerpo de diff, sólo `binary: true` y tamaños.
- la redacción consciente de secretos del stream se aplica **antes** de emitir, igual que al resto.
- si generar el diff falla o excede presupuesto de tiempo, se emite `file.diff` con `unavailable` y motivo; **MUST NOT** bloquear ni retrasar la herramienta.

### 3.4 `agent.node`

`agent.state` es hoy de nivel de job y ya transporta `kind:'graph'` y `graph:{nodes,edges}`. Los nodos individuales son invisibles, lo que deja el Graph como caja negra en la línea temporal.

`agent.node` transporta `agent_id` del job padre, `node_id`, `status`, y temporización. El contenido de inferencia del nodo **MUST NOT** incluirse: el stream es operativo y auditable, no razonamiento del modelo.

Si el Agent Provider no reporta transiciones por nodo, el Core emite al menos entrada y salida derivadas de su propio despacho, y marca `source: 'derived'`. Inventar estados intermedios que el backend no reportó está prohibido.

### 3.5 `tool.progress` y aprobaciones

`tool.progress` se emite sólo cuando el origen real produce progreso — conectores MCP vía `onprogress`, procesos de larga duración. **MUST NOT** sintetizarse progreso artificial ni barras de avance estimadas.

**Aprobaciones — decisión de alcance.** El doc de arquitectura pide "approval requests/decisions" en el stream, pero el Core no tiene hoy ningún mecanismo de aprobación y añadirlo sería una superficie de autoridad nueva y no trivial. Decisión: en fase 1 las aprobaciones son **exclusivamente del broker Desktop**, para operaciones de ciclo de vida del host (instalar, reparar, actualizar, rollback, borrar distro). Se emiten en el stream local del Desktop, no por `/events` del Core. Una superficie de aprobación en el Core queda **diferida y fuera de este cambio**; introducirla exigirá su propio análisis de autoridad.

### 3.6 Coste y contrapresión

Los eventos nuevos aumentan volumen. El buffer de replay (2000 eventos) se llenaría antes y acortaría la ventana de reconexión.

Mitigaciones obligatorias: los eventos de diff y de progreso cuentan contra el mismo límite pero son **descartables** — ante presión, se descartan antes que `tool.*`, `process.exit` o `stream.*`, y el descarte se señaliza explícitamente. Un consumidor **MUST** poder distinguir "no hubo diff" de "el diff se descartó por presión".

## 4. Negociación de contrato

El broker Rust negocia **antes** de habilitar UI dependiente de capacidades:

1. `harness_contract_v1` sobre el endpoint autenticado.
2. Exigir `protocol == "rhodiz-arnes"` y `contract_version == 1`.
3. Sondear nombres de herramientas o comparar cadenas de versión de servicio **MUST NOT** aceptarse como sustituto.
4. Ante contrato ausente, ilegible o incompatible: fallar cerrado con diagnóstico accionable; **MUST NOT** degradar a un modo adivinado.

Las sesiones de programación normales se abren con `capability_profile: "coding"`. El valor por defecto del Core sigue siendo `full` por compatibilidad con clientes Route existentes; el Desktop no pide autoridad que no necesita. El lease V2 devuelto contiene el manifiesto de capacidad firmado que gobierna cada `session_*` posterior.

Diagnostics consume `arnes_doctor_v1` y `provider_doctor_v1` y renderiza sus comprobaciones estructuradas. Son señales de soporte en runtime, **no** certificación de release.

## 5. Ciclo de vida del runtime gestionado

El broker Rust posee todas las operaciones de host como comandos tipados. No existe terminal WSL arbitraria expuesta por IPC.

Secuencia de instalación: verificar disponibilidad y versión de WSL → aprovisionar/importar la distro `RHODIZ-Harness` → asegurar systemd → instalar Docker Engine y plugin Compose dentro de la distro → materializar el bundle de runtime versionado con digests fijados → arrancar por servicio systemd de bootstrap → verificar el conjunto de servicios de primera parte → negociar contrato Core → abrir UI.

Rutas que deben tratarse explícitamente, no asumirse ausentes: WSL ausente, WSL presente, WSL desactualizado, reinicio con recuperación de backend, fallo del demonio Docker y reparación.

Exposición por defecto: sólo loopback. Route, Memory y sidecars **MUST NOT** publicar puertos hacia Windows; el Core es su cliente acotado.

## 6. Integridad de release

Un release coordinado se describe con `release.json` conforme a `config/release-manifest.schema.json`, más una firma separada `release.json.sig`.

- El broker **SHALL** verificar la firma sobre los **bytes exactos** del manifiesto **antes** de parsear cualquier URL, versión o digest. Parsear primero y verificar después es un fallo de diseño, no una optimización.
- Etiquetas `:latest` **MUST NOT** aceptarse.
- `release_sequence` monótono es la **única** autoridad anti-rollback. SemVer **MUST NOT** usarse como guarda de degradación.
- Flujo del updater de runtime: verificar manifiesto → verificar digests de artefactos e imágenes → preparar N+1 → migrar → arrancar → negociar contrato Core → doctor y smoke en vivo → activar atómicamente. El release N anterior permanece disponible para rollback acotado cuando manifiesto y política lo permitan.
- La capa ejecutable del Desktop usa el updater de Tauri 2 con su artefacto firmado; es un eje **independiente** del updater del runtime WSL/Docker.

## 7. Frontera de seguridad del Desktop

El estado `0.1.0` ya establece lo esencial y **se preserva**: renderer local con CSP estricta, DevTools desactivado en producción, sin plugin genérico de shell/proceso, comandos ACL generados por `AppManifest` y concedidos por capacidad Tauri explícita.

Sobre esa base:

- Cada comando IPC nuevo se añade al allowlist con argumentos tipados y validados. Un argumento **MUST NOT** transportar ruta ejecutable, argv, ni selector de autoridad.
- Los secretos viven en almacén de credenciales del SO o en fichero de secreto propiedad de root dentro de WSL, consumidos sólo por broker y runtime.
- La navegación del renderer queda anclada a orígenes empaquetados/locales.
- Los logs redactan credenciales, bearers y salida privada de modelo.
- La eliminación o reinicialización de la distro exige acción explícita del usuario y una ruta documentada de respaldo/exportación.

## 8. Procedencia

Los tres pins MIT (`openhands`, `onyx-foss`, `librechat`) son anclas inmutables, no dependencias flotantes. Hoy `PROVENANCE.md` declara "None yet" en código importado.

Por cada componente importado se registra: repositorio y commit upstream, ruta original, licencia/copyright, destino local, resumen de modificación, resultado de revisión de seguridad y cobertura de tests. Ningún backend de esos proyectos es autoridad de ejecución.

## 9. Riesgos reconocidos

- **El E2E de Route usó identidad OAuth sintética.** El Desktop no debe presumir que el camino de host aprobado está certificado; esa tarea vive en el plan `remote-access` y está bloqueada arriba por `403 Trusted Hosts`.
- **La certificación Windows no puede hacerse aquí.** El desarrollo ocurre en Linux; una comprobación cruzada en Linux **MUST NOT** contar como evidencia Windows. Las tareas marcadas Windows quedan abiertas hasta ejecutarse en hardware/VM Windows real.
- **`per_session_cgroup` es `false`.** Ninguna UI puede presentar el disco por sesión como aislamiento duro.
- **Volumen del stream.** Sin la política de descarte de §3.6, los diffs pueden acortar la ventana de replay y degradar la reconexión — el problema que el stream pretende resolver.
