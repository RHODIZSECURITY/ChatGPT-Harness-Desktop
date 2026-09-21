## Purpose

Definir cómo el broker Rust instala, verifica, repara y opera el runtime Linux gestionado dentro de WSL2 sin conceder autoridad genérica de host al renderer.

## ADDED Requirements

### Requirement: La distro gestionada tiene un nombre canónico único
La distribución WSL2 gestionada SHALL llamarse `RHODIZ-Harness` en instalador, broker, documentación y diagnósticos.

#### Scenario: Un documento usa un nombre distinto
- **WHEN** una superficie del producto referencia otro nombre de distro
- **THEN** se trata como defecto y se corrige al nombre canónico

### Requirement: Las operaciones de host son comandos tipados
Instalación, verificación, reparación, arranque, parada, logs, actualización y rollback SHALL implementarse como comandos Rust tipados del broker, y MUST NOT convertirse en herramientas MCP del Agent ni exponerse como terminal WSL arbitraria por IPC.

#### Scenario: Intento de ejecutar un comando arbitrario por IPC
- **WHEN** el renderer intenta invocar una cadena de comando no prevista en el allowlist
- **THEN** la invocación se rechaza

#### Scenario: Argumento que transporta ejecutable o argv
- **WHEN** un argumento de IPC contiene ruta ejecutable, argv o selector de autoridad
- **THEN** la validación lo rechaza antes de cualquier efecto

### Requirement: Las rutas de WSL se tratan explícitamente
El broker SHALL cubrir de forma explícita los estados WSL ausente, WSL presente y WSL desactualizado, con resultado fail-closed y diagnóstico accionable en cada uno.

#### Scenario: WSL desactualizado
- **WHEN** la versión de WSL no alcanza el mínimo del manifiesto
- **THEN** la instalación se detiene con diagnóstico y no deja un runtime a medias

### Requirement: El backend se expone sólo en loopback
Los servicios del runtime gestionado SHALL exponerse sólo en loopback; Route, Memory MCP y sidecars de provider MUST NOT publicar puertos hacia Windows por defecto, y el producto MUST NOT requerir exposición LAN.

#### Scenario: Inspección de puertos tras la instalación
- **WHEN** se inspeccionan los puertos publicados hacia Windows
- **THEN** sólo aparecen los estrictamente requeridos por el broker, y ninguno de Route, Memory o sidecars

### Requirement: El harness no recibe docker.sock
El Desktop MUST NOT montar `docker.sock` dentro del contenedor del harness; Docker permanece como detalle de implementación del host WSL dedicado.

#### Scenario: Revisión de la composición desplegada
- **WHEN** se inspecta la composición que el Desktop arranca
- **THEN** el contenedor del harness no tiene `docker.sock` montado

### Requirement: El runtime se recupera de fallos operativos
El broker SHALL recuperar el backend tras reinicio de Windows y tras fallo del demonio Docker.

#### Scenario: Reinicio de Windows con sesiones previas
- **WHEN** el usuario reinicia y vuelve a abrir la aplicación
- **THEN** el runtime se restablece y los worktrees de Session previos siguen disponibles para reanudar

#### Scenario: Demonio Docker caído
- **WHEN** el demonio Docker no responde
- **THEN** el estado se reporta como fallo accionable y la reparación es una operación explícita, no automática y silenciosa

### Requirement: La eliminación de datos es explícita
Desinstalar o reinicializar la distro SHALL exigir acción explícita del usuario y ofrecer una ruta documentada de respaldo o exportación de los datos de Project.

#### Scenario: Desinstalación con Projects existentes
- **WHEN** el usuario desinstala con datos de Project presentes
- **THEN** se preservan o se exportan explícitamente, y nunca se descartan de forma silenciosa
