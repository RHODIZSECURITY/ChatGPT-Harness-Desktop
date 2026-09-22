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

El script no le pregunta nada a la máquina: no añade nombre de host, ni nombre
de usuario, ni rutas propias, y el único texto que inventa es el `-Label`.

Pero guarda lo que imprime `wsl.exe`, y `wsl.exe` cuenta más de lo que sugiere
su nombre. La captura real de `evidence/windows/wsl-probe-win11-home-26200.json`
revela:

- la compilación exacta de Windows y su UBR (`Versión de Windows: 10.0.26200.9457`)
- las versiones del kernel WSL y de sus componentes (WSLg, MSRDC, Direct3D,
  DXCore), una de las cuales lleva una cadena de rama de servicing
- el nombre de la distribución por defecto, vía `--status` (`Ubuntu`)
- el idioma del sistema, porque `wsl.exe` traduce sus propias etiquetas

Nada de eso es un secreto, y todo ello es justamente lo que convierte la captura
en evidencia sobre un Windows concreto en vez de sobre una forma abstracta. Aun
así es más que "nada": **lee el fichero antes de commitearlo** y decide tú. Lo
que no estés dispuesto a publicar, no lo commitees.

Una versión anterior de este documento afirmaba que no se recogía la compilación
de Windows. Era falso, y la primera captura real lo demostró.

Si `wsl.exe` no existe en el host, eso también se registra: la ruta "WSL
ausente" es una ruta certificable (tarea 10.2), no un fallo de la captura.

## Verificación (aquí, en CI)

`src-tauri/broker-core/tests/wsl_evidence_replay.rs` reproduce cada captura
contra `decode_utf16le` y `extract_wsl_version`. Si los bytes reales no son el
UTF-16LE que el broker supone, o no contiene un triple extraíble, el test
falla con los primeros bytes en hexadecimal.

Sin capturas el test informa y pasa, porque la evidencia se produce a mano en
una máquina Windows y no puede fabricarse desde aquí. Ya existe una captura
commiteada, así que `RHODIZ_REQUIRE_WSL_EVIDENCE=1` está activado a nivel de
workflow en `.github/workflows/ci.yml`: borrar la captura pone CI en rojo en
vez de devolvernos en silencio a "sin evidencia, sin opinión".

### Resultado sobre la primera captura real

`evidence/windows/wsl-probe-win11-home-26200.json`, Windows 11 Home build
26200, WSL 2.7.10.0, sistema en español. El replay decodifica los 490 bytes de
`--version` y extrae `2.7.10` (suficiente frente a `MINIMUM_WSL_VERSION`
2.0.0). Tres cosas que los fixtures sintéticos no cubrían:

1. **La salida real NO lleva BOM.** La rama `FF FE` de `decode_utf16le` es
   defensiva, no el camino observado. Se mantiene igualmente.
2. **`wsl.exe` traduce sus etiquetas.** Este host imprime `Versión de WSL:`,
   no `WSL version:`. Un parser anclado a la etiqueta inglesa habría extraído
   nada y habría bloqueado un WSL perfectamente válido. `extract_wsl_version`
   busca el primer triple con puntos, no una etiqueta, y por eso sobrevive;
   `version_extraction_survives_a_localized_windows` fija esa propiedad.
3. **Hay seis líneas de versión más**, cada una con sus propios números con
   puntos, y el escaneo se queda con la primera. Que la versión de WSL sea
   siempre la primera es lo único que sigue siendo suposición: se cumplió en
   este host y ningún contrato documentado lo garantiza.

## Ejecutar la verificación completa en el host Windows

```powershell
npm ci
npm run verify:windows
```

### Prerrequisito que CI nunca reproduce

`winget install Rustlang.Rustup` deja `stable`, pero el repositorio fija
1.98.1. Al entrar al checkout, rustup instala esa toolchain automáticamente
**sin `rustfmt` ni `clippy`**, y `verify:windows` muere en su primer paso de
Rust:

```
error: 'cargo-fmt.exe' is not installed for the toolchain '1.98.1-x86_64-pc-windows-msvc'
```

```powershell
rustup component add --toolchain 1.98.1-x86_64-pc-windows-msvc rustfmt clippy
```

Los runners de CI no lo reproducen nunca: `.github/workflows/ci.yml` instala la
toolchain con `--component rustfmt --component clippy` explícitos. Es decir, un
CI verde no dice nada sobre si una máquina limpia puede ejecutar el gate.

### Qué NO cubre `verify:windows`

`verify:windows` es `verify:portable` + `cargo check --tests` + `clippy`. **No
ejecuta `tauri build`**: no produce binario enlazado de la aplicación ni
instalador. Un `verify:windows` verde no toca la tarea 10.9 (instalador, firma
de código, E2E completo de Windows).

Ejecutado sobre 4e8cbc2 en Windows 11 Home 26200 en español, con
`RHODIZ_REQUIRE_WSL_EVIDENCE=1`: EXIT=0, 45 tests de Rust en verde (incluidos
los 2 de replay), clippy limpio con `-D warnings`.

## Qué certifica y qué no

Certifica la codificación y el barrido de versión sobre bytes reales. **No**
certifica el resto de la sección 10: instalación limpia, reinicio con
recuperación, fallo del demonio Docker, rollback, desinstalación, exposición
loopback bajo VPN ni firma del instalador siguen requiriendo observación humana
sobre la máquina.
