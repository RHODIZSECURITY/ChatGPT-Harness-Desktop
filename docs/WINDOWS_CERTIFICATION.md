# Certificación Windows

La sección 10 del plan OpenSpec no admite evidencia obtenida en Linux. Este
documento describe cómo se produce esa evidencia de forma reproducible y
verificable, en lugar de pegar salidas sueltas en una conversación.

## Por qué existe este arnés

El broker decodifica la salida de `wsl.exe` como UTF-16LE y busca en ella un
triple `mayor.menor.parche`. Hasta ahora ambas conductas estaban certificadas
únicamente contra bytes que este repositorio sintetiza en sus propios tests: la
forma real en bytes de `wsl.exe --version` nunca se había observado. Un test
que sólo ve sus propias suposiciones no certifica nada sobre Windows.

## Captura (en el host Windows)

```powershell
powershell -ExecutionPolicy Bypass -File scripts\windows-evidence\probe-wsl.ps1 -Label win11-24h2
```

El script ejecuta las dos invocaciones que el broker realiza de verdad
(`wsl.exe --version` y `wsl.exe --status`) y guarda el **flujo de bytes en
crudo** en base64, junto con el código de salida. No decodifica a texto: todo
lo que pasa por una cadena en PowerShell se recodifica, y eso destruiría
exactamente la propiedad bajo prueba.

El resultado se escribe en `evidence/windows/wsl-probe-<label>.json`.

### Qué se recoge y qué no

No se recoge nombre de máquina, nombre de usuario, rutas ni número de
compilación de Windows. El único texto identificativo que entra en el fichero
es el `-Label` que elige el operador.

`wsl.exe --status` sí imprime el nombre de la distribución por defecto.
**Revisa el fichero antes de commitearlo.**

Si `wsl.exe` no existe en el host, eso también se registra: la ruta "WSL
ausente" es una ruta certificable (tarea 10.2), no un fallo de la captura.

## Verificación (aquí, en CI)

`src-tauri/broker-core/tests/wsl_evidence_replay.rs` reproduce cada captura
contra `decode_utf16le` y `extract_wsl_version`. Si los bytes reales no son el
UTF-16LE que el broker supone, o no contiene un triple extraíble, el test
falla con los primeros bytes en hexadecimal.

Sin capturas el test informa y pasa, porque la evidencia se produce a mano en
una máquina Windows y no puede fabricarse desde aquí. En cuanto exista una
captura commiteada, `RHODIZ_REQUIRE_WSL_EVIDENCE=1` convierte su ausencia en
fallo, para que la puerta no pueda degradarse en silencio a "sin evidencia,
sin opinión".

## Qué certifica y qué no

Certifica la codificación y el barrido de versión sobre bytes reales. **No**
certifica el resto de la sección 10: instalación limpia, reinicio con
recuperación, fallo del demonio Docker, rollback, desinstalación, exposición
loopback bajo VPN ni firma del instalador siguen requiriendo observación humana
sobre la máquina.
