## Purpose

Definir lo que el Harness Core debe publicar y sostener para que el cliente Desktop pueda decidir, de forma verificable, qué superficie habilitar.

## ADDED Requirements

### Requirement: La habilitación de UI exige negociación de contrato
El broker Desktop SHALL negociar `harness_contract_v1` y exigir protocolo `rhodiz-arnes` con `contract_version` 1 antes de habilitar UI dependiente de capacidades.

#### Scenario: Contrato incompatible
- **WHEN** el Core responde con un protocolo o versión de contrato distintos de los exigidos
- **THEN** el cliente falla cerrado con diagnóstico accionable y MUST NOT degradar a un modo adivinado

#### Scenario: Contrato ausente o ilegible
- **WHEN** `harness_contract_v1` no está disponible o no puede parsearse
- **THEN** la UI dependiente de capacidades permanece deshabilitada

### Requirement: El sondeo de herramientas no sustituye al contrato
Sondear nombres de herramientas o comparar cadenas de versión de servicio MUST NOT aceptarse como sustituto de la negociación de contrato.

#### Scenario: El cliente deduce capacidades del catálogo
- **WHEN** el cliente intenta inferir compatibilidad a partir de `tools/list` o de la versión de servicio
- **THEN** esa inferencia se rechaza como base de habilitación

### Requirement: Las sesiones Desktop piden el mínimo de autoridad
Las sesiones de programación normales del Desktop SHALL abrirse con `capability_profile: "coding"`; el valor por defecto `full` del Core se conserva por compatibilidad con clientes Route existentes.

#### Scenario: Sesión abierta con autoridad excesiva
- **WHEN** el Desktop abre una sesión de programación con perfil `full` sin necesidad demostrada
- **THEN** se trata como defecto de autoridad y la sesión se reabre con `coding`

#### Scenario: El perfil coding deniega una operación no concedida
- **WHEN** una llamada `session_*` no está en el manifiesto de capacidad firmado del lease
- **THEN** la llamada se rechaza centralmente antes de cualquier efecto

### Requirement: El lease firmado es la única autoridad de sesión
Sólo un `workspace_lease` V2 emitido por el servidor SHALL autorizar herramientas `session_*`; un `project_id`, `session_id` o ruta suministrados por el cliente MUST NOT constituir autoridad.

#### Scenario: Lease manipulado
- **WHEN** se presenta un lease con firma alterada
- **THEN** la operación falla cerrada

#### Scenario: Lease de generación anterior tras rotación
- **WHEN** se usa un lease cuya generación fue rotada por `workspace_session_resume`
- **THEN** la operación se rechaza por lease rotado o revocado

### Requirement: El lease nunca se persiste en el cliente
El `workspace_lease` MUST NOT escribirse en `localStorage`, `IndexedDB`, parámetros de URL, logs, memoria durable, ficheros de repositorio ni commits.

#### Scenario: Auditoría de persistencia del renderer
- **WHEN** se inspeccionan los almacenes del renderer tras una sesión de programación
- **THEN** no existe ningún lease persistido

### Requirement: Las superficies doctor son soporte, no certificación
`arnes_doctor_v1` y `provider_doctor_v1` SHALL exponer comprobaciones estructuradas que distingan fallo de omisión, y MUST NOT presentarse como certificación de release.

#### Scenario: Doctor en verde con digests sin verificar
- **WHEN** los doctores reportan `ok` pero los digests de imagen no se han verificado
- **THEN** el release no se considera certificado
