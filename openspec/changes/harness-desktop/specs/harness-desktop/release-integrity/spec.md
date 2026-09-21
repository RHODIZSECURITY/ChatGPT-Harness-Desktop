## Purpose

Definir cómo se describe, verifica y revierte un release coordinado del Desktop y del runtime gestionado, de forma que ninguna actualización de backend pueda aplicarse sin evidencia criptográfica.

## ADDED Requirements

### Requirement: La firma se verifica antes de parsear
El broker SHALL verificar la firma separada sobre los bytes exactos del manifiesto antes de parsear cualquier URL, versión o digest contenidos en él.

#### Scenario: Manifiesto con firma inválida
- **WHEN** la firma no valida sobre los bytes exactos
- **THEN** el manifiesto se descarta sin parsearse y no se contacta ninguna URL que contenga

### Requirement: Los releases certificados fijan digests exactos
Un manifiesto de release SHALL fijar digests de imagen para Core, Route, Memory MCP y sidecars habilitados, más versión de esquema de Compose/config, versión de migración y mínimos de WSL y Desktop; las etiquetas `:latest` MUST NOT aceptarse.

#### Scenario: Manifiesto candidato con etiqueta flotante
- **WHEN** un manifiesto referencia `:latest` en cualquier imagen
- **THEN** la puerta de certificación lo rechaza

### Requirement: La secuencia de release es la autoridad anti-rollback
Un `release_sequence` monótono SHALL ser la única autoridad anti-degradación; SemVer MUST NOT usarse como guarda de rollback.

#### Scenario: Manifiesto con versión mayor y secuencia anterior
- **WHEN** se ofrece un manifiesto con SemVer superior pero secuencia inferior a la activa
- **THEN** se rechaza como degradación

### Requirement: La actualización del runtime es transaccional
La actualización SHALL seguir el orden verificar manifiesto, verificar digests, preparar N+1, migrar, arrancar, negociar contrato Core, ejecutar doctor y smoke en vivo, y activar atómicamente.

#### Scenario: El contrato falla tras arrancar el candidato
- **WHEN** la negociación del contrato Core falla sobre el release N+1
- **THEN** la activación no ocurre y el release N permanece activo

#### Scenario: Rollback tras candidato defectuoso
- **WHEN** un candidato deliberadamente malo se despliega y falla sus comprobaciones
- **THEN** el sistema vuelve al release N dentro de los límites que permiten manifiesto y política de migración

### Requirement: Los ejes de actualización son independientes
El updater del ejecutable Desktop SHALL permanecer independiente del updater del runtime WSL/Docker.

#### Scenario: Actualización sólo del ejecutable
- **WHEN** se actualiza la capa Tauri sin cambio de manifiesto de runtime
- **THEN** el runtime gestionado no se altera
