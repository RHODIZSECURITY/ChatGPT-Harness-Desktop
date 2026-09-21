## Purpose

Preservar y extender la frontera de seguridad que el cliente ya estableció en `0.1.0`, de forma que añadir funcionalidad no erosione el aislamiento entre renderer, broker y runtime.

## ADDED Requirements

### Requirement: El renderer no recibe autoridad genérica
El renderer MUST NOT recibir autoridad genérica de PowerShell, `wsl.exe`, Docker, filesystem ni proceso; invoca exclusivamente un allowlist tipado de comandos del broker.

#### Scenario: Se añade un comando IPC nuevo
- **WHEN** una funcionalidad nueva requiere una operación de host
- **THEN** se añade como comando tipado con argumentos validados y concesión explícita de capacidad, nunca como pasarela genérica

#### Scenario: Plugin genérico de shell o proceso
- **WHEN** se propone habilitar un plugin de shell o proceso de Tauri
- **THEN** se rechaza

### Requirement: Los secretos no alcanzan el renderer
Los bearers del Harness y las credenciales de backend SHALL residir en almacén de credenciales del sistema operativo o en fichero de secreto propiedad de root dentro de WSL, consumidos sólo por broker y runtime.

#### Scenario: Inspección del renderer en ejecución
- **WHEN** se auditan el almacenamiento y la memoria accesible del renderer
- **THEN** no existe ningún bearer ni credencial de backend

#### Scenario: Registro de diagnóstico
- **WHEN** se emiten logs de diagnóstico o soporte
- **THEN** credenciales, bearers y salida privada de modelo aparecen redactados

### Requirement: No hay ejecución ni navegación remota
La aplicación SHALL aplicar una Content Security Policy sin ejecución de código remoto y anclar la navegación del renderer a orígenes empaquetados o locales; las DevTools de producción SHALL permanecer desactivadas salvo build de diagnóstico explícito.

#### Scenario: Intento de navegación a un origen remoto
- **WHEN** el renderer intenta navegar a un origen no empaquetado
- **THEN** la navegación se bloquea

### Requirement: El aislamiento Project/Session se preserva desde el Desktop
La cadena `tenant -> principal -> Project -> Session -> lease firmado` SHALL preservarse desde el camino Desktop, sin autoridad cruzada de ruta o de proceso entre Projects o Sessions.

#### Scenario: Dos sesiones concurrentes sobre el mismo Project
- **WHEN** el Desktop abre dos sesiones simultáneas sobre un mismo Project
- **THEN** cada una opera en su worktree aislado y ninguna alcanza el del otro por ruta absoluta

#### Scenario: Regresión de Landlock desde el camino Desktop
- **WHEN** una shell de sesión lanzada desde el Desktop intenta abrir `/run/secrets`, `/state` o el worktree de otra sesión
- **THEN** el acceso se deniega

### Requirement: Las capacidades no configuradas fallan cerrado
Las capacidades de provider sin backend aprobado SHALL fallar cerrado, y la UI MUST NOT fabricar resultados, sustituir backend ni presentar como disponible una capacidad que no lo está.

#### Scenario: Búsqueda web sin backend configurado
- **WHEN** el usuario invoca una capacidad cuyo backend no está configurado
- **THEN** la UI reporta indisponibilidad explícita y no devuelve resultados inventados

#### Scenario: Agentes sin proveedor de inferencia
- **WHEN** no hay proveedor de agentes configurado
- **THEN** la UI MUST NOT afirmar que hay agentes ejecutándose en segundo plano

### Requirement: El almacenamiento de Project no se comparte en silencio
Las carpetas de Windows SHALL tratarse como origen explícito de importación o exportación, y MUST NOT convertirse en un workspace compartido escribible de forma silenciosa.

#### Scenario: El usuario quiere trabajar sobre una carpeta de Windows
- **WHEN** se selecciona una carpeta del host
- **THEN** la acción disponible es importar o exportar explícitamente, no montar un workspace compartido
