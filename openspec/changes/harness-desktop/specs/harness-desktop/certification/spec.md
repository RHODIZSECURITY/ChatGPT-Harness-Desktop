## Purpose

Definir qué evidencia cuenta como certificación del frente Desktop y qué no, para que ninguna afirmación de completitud se apoye en comprobaciones de alcance insuficiente.

## ADDED Requirements

### Requirement: La evidencia Windows exige entorno Windows
Las puertas de certificación Windows SHALL ejecutarse en un entorno Windows real; una comprobación cruzada en Linux MUST NOT tratarse como evidencia equivalente.

#### Scenario: Puertas portables verdes en Linux
- **WHEN** `verify:portable` pasa en Linux
- **THEN** las tareas de certificación Windows permanecen abiertas

### Requirement: El alcance de la verificación iguala al alcance de la afirmación
Una afirmación de completitud SHALL sostenerse sobre verificación del mismo alcance: un test unitario prueba su unidad, una comprobación de sintaxis prueba sintaxis, un smoke local prueba el camino local, y un catálogo interno de herramientas prueba registro, no ejecución externa exitosa.

#### Scenario: Catálogo de herramientas usado como prueba de integración
- **WHEN** se ofrece el número de herramientas anunciadas como evidencia de que una integración externa funciona
- **THEN** esa evidencia se rechaza por alcance insuficiente

#### Scenario: Ausencia de fallos presentada como perfección
- **WHEN** no se detectan problemas en una comprobación
- **THEN** se declara exactamente qué se probó y qué queda sin verificar

### Requirement: Las puertas de ambos repositorios deben estar verdes
La aceptación final SHALL exigir puertas locales y CI de SHA exacto en verde en `ChatGPT-Arnes` y en `ChatGPT-Harness-Desktop` sobre el mismo candidato certificado.

#### Scenario: Un repositorio en verde y el otro no
- **WHEN** sólo uno de los dos repositorios alcanza sus puertas
- **THEN** el frente no se considera completo

### Requirement: El baseline certificado no puede quedar debilitado
La aceptación final SHALL verificar que ninguna obligación de `baseline-obligations.md` fue debilitada, eludida o reinterpretada como opcional.

#### Scenario: Una funcionalidad Desktop relaja la composición Docker
- **WHEN** una necesidad del instalador propone quitar `no-new-privileges`, el rootfs de sólo lectura o el descarte de capacidades
- **THEN** la propuesta se rechaza salvo autorización explícita del operador con actualización del baseline en el mismo commit

### Requirement: La finalización exige autorización del operador
El frente MUST NOT declararse `done`, `10/10`, `production complete` ni `commercial-ready` sin autorización explícita del operador.

#### Scenario: Todas las tareas marcadas y puertas verdes
- **WHEN** las tareas están completas y las puertas en verde
- **THEN** el estado se reporta como listo para autorización, no como release realizado
