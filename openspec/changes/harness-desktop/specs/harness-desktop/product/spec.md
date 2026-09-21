## Purpose

Definir la frontera de producto del frente RHODIZ Harness Desktop: qué es el cliente, qué no puede llegar a ser, y cómo se relaciona con el Harness Core y con el plan RemoteAccess de Route.

## ADDED Requirements

### Requirement: El Desktop no es una segunda autoridad de ejecución
El cliente Desktop, su renderer y su broker Rust SHALL presentar estado y mediar llamadas autorizadas, y MUST NOT ejecutar efectos de Project/Session por su cuenta ni reimplementar la semántica de programación del Core.

#### Scenario: El cliente intenta un efecto de workspace sin pasar por el Core
- **WHEN** una ruta del Desktop intenta escribir, parchear o ejecutar sobre un Project sin una operación MCP con lease firmado
- **THEN** la operación se rechaza y el efecto no ocurre

#### Scenario: Traducción a shell cruda existiendo operación equivalente
- **WHEN** el usuario realiza una acción para la que existe una operación MCP con lease
- **THEN** el cliente usa esa operación y MUST NOT traducirla a un comando de shell arbitrario

### Requirement: La hoja de ruta Desktop es única
Este cambio SHALL ser la hoja de ruta canónica del frente Desktop, y ningún agente MUST crear o seguir una hoja de ruta Desktop competidora.

#### Scenario: Aparece un plan Desktop paralelo
- **WHEN** un agente propone un roadmap Desktop fuera de `openspec/changes/harness-desktop/`
- **THEN** ese plan se rechaza y el trabajo se reconduce a este cambio

### Requirement: RemoteAccess no se duplica aquí
Este cambio MUST NOT duplicar, bifurcar ni contradecir `openspec/changes/remote-access/` del repositorio de Route; la clasificación de Cloud ChatGPT Arnes como perfil MCP downstream dentro del Client de Route SHALL respetarse sin cambios.

#### Scenario: Una tarea Desktop toca identidad remota u OAuth
- **WHEN** una tarea requiere cambiar identidad remota, OAuth, Relay, pairing o grants
- **THEN** esa tarea pertenece al plan `remote-access` y MUST NOT ejecutarse bajo este cambio

#### Scenario: Se confunde certificación Desktop con certificación RemoteAccess
- **WHEN** el frente Desktop alcanza sus puertas
- **THEN** ese resultado MUST NOT presentarse como certificación de RemoteAccess, ni el inverso

### Requirement: Route y Memory no conceden autoridad de workspace
RHODIZ MCP Route y RHODIZ Memory MCP SHALL tratarse como servicios de primera parte del runtime gestionado; un Route o Memory sanos MUST NOT habilitar por sí mismos autoridad de workspace.

#### Scenario: Route sano con contrato Core ausente
- **WHEN** Route responde sano pero la negociación del contrato Core falla
- **THEN** la UI dependiente de capacidades permanece deshabilitada
