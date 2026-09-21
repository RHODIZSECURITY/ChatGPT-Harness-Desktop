## Purpose

Definir el stream de actividad operativa que el Core debe emitir para que el Desktop muestre evidencia en vivo. El documento de arquitectura lo declara requisito de producto, no telemetría opcional.

## ADDED Requirements

### Requirement: El stream es operativo y auditable, no razonamiento
Los eventos SHALL describir efectos observables — herramientas, procesos, ficheros, Git, agentes, continuidad — y MUST NOT transportar razonamiento oculto del modelo ni contenido de inferencia de nodos.

#### Scenario: Un nodo de Graph produce texto de inferencia
- **WHEN** un nodo del Graph genera salida de modelo
- **THEN** el evento de transición reporta estado y temporización, y el contenido de inferencia no se incluye

### Requirement: La extensión del stream es aditiva
Los tipos de evento ya existentes SHALL conservar nombre, forma y semántica; los tipos nuevos se añaden sin retirar campos, y un consumidor que reciba un `kind` desconocido MUST ignorarlo en lugar de fallar.

#### Scenario: Consumidor del conjunto anterior tras la extensión
- **WHEN** un cliente escrito contra el conjunto de 15 tipos se conecta al stream extendido
- **THEN** sigue funcionando sin cambios

### Requirement: Los cambios de fichero declaran su operación
`file.change` SHALL incluir el tipo de operación `create`, `update`, `delete` o `move`, derivado de la existencia previa del destino y MUST NOT derivarse del nombre de la herramienta.

#### Scenario: Escritura sobre fichero inexistente
- **WHEN** una escritura crea un fichero que no existía
- **THEN** la operación reportada es `create` y no `update`

### Requirement: Los diffs viajan en un evento separado y acotado
El diff SHALL emitirse como evento `file.diff` independiente, con límite de bytes configurable, marca de truncamiento, manejo explícito de binarios y redacción aplicada antes de la emisión.

#### Scenario: Diff que excede el límite
- **WHEN** el diff supera el límite configurado
- **THEN** se emite con `truncated` verdadero y sólo el resumen estadístico

#### Scenario: Fichero binario modificado
- **WHEN** el fichero modificado es binario
- **THEN** no se emite cuerpo de diff y se marca como binario con sus tamaños

#### Scenario: La generación del diff falla o excede presupuesto
- **WHEN** el diff no puede generarse dentro del presupuesto de tiempo
- **THEN** se emite un resultado no disponible con motivo, y la herramienta originaria no se bloquea ni se retrasa

### Requirement: Las transiciones de nodo del Graph son observables
El Core SHALL emitir transiciones por nodo del Graph; cuando el backend no las reporte, el Core emite al menos entrada y salida derivadas de su propio despacho y las marca como derivadas.

#### Scenario: Backend sin reporte por nodo
- **WHEN** el Agent Provider no informa estados intermedios de nodo
- **THEN** el Core marca los eventos como derivados y MUST NOT inventar estados que el backend no reportó

### Requirement: Los checkpoints aparecen en la línea temporal
La creación de un checkpoint de continuidad SHALL emitir su propio evento con identificadores suficientes para recuperar el checkpoint completo, sin incrustar su cuerpo en el evento.

#### Scenario: Checkpoint creado durante una tarea larga
- **WHEN** se crea un checkpoint
- **THEN** la línea temporal muestra el hito y permite recuperar el detalle por identificador

### Requirement: El progreso reportado es real
`tool.progress` SHALL emitirse únicamente a partir de progreso real del origen, y MUST NOT sintetizarse progreso artificial ni estimaciones de avance.

#### Scenario: Herramienta sin señal de progreso
- **WHEN** una herramienta de larga duración no expone progreso
- **THEN** no se emiten eventos de progreso para ella

### Requirement: La actividad LSP es observable
Las operaciones LSP de diagnósticos, símbolos, definición, referencias y planificación de rename SHALL emitir actividad, conservando que el rename devuelve ediciones y MUST NOT aplicarlas.

#### Scenario: Rename planificado desde el Desktop
- **WHEN** se solicita una planificación de rename
- **THEN** se emiten las ediciones previstas y ningún fichero se modifica

### Requirement: La contrapresión no sacrifica eventos de autoridad
Bajo presión de buffer, los eventos de diff y de progreso SHALL descartarse antes que los eventos de herramienta, fin de proceso y control de stream; el descarte SHALL señalizarse.

#### Scenario: Buffer bajo presión durante una sesión intensa
- **WHEN** el volumen supera la capacidad del buffer de replay
- **THEN** se descartan diffs y progreso, nunca `process.exit` ni `stream.*`

#### Scenario: El consumidor distingue ausencia de descarte
- **WHEN** un cambio de fichero no trae diff asociado
- **THEN** el consumidor puede determinar si no hubo diff o si fue descartado por presión

### Requirement: El stream nunca transporta secretos
Ningún evento MUST contener bearers, credenciales de provider, secretos del entorno hijo ni contenido de ficheros de secreto, incluido el caso de un diff sobre un fichero que los contenga.

#### Scenario: Diff sobre un fichero con un secreto
- **WHEN** se modifica un fichero que contiene material sensible
- **THEN** la redacción se aplica antes de emitir y el secreto no aparece en el stream

### Requirement: La reconexión preserva orden y recuperación
La reconexión por `runtime_id` más secuencia SHALL mantenerse, emitiendo reinicio de stream ante cambio de runtime, cursor futuro o ventana de replay excedida; la recuperación durable proviene del estado y los checkpoints del Core, no de la memoria del renderer.

#### Scenario: El Core se reinicia durante una sesión
- **WHEN** el proceso del Core cambia y el `runtime_id` ya no coincide
- **THEN** se emite reinicio de stream y el cliente reconstruye estado desde las APIs de Project/Session/checkpoint
