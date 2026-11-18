# Línea de comandos

[简体中文](zh-CN.md) · [English](en.md) · [日本語](ja.md) · Español · [README](../readme/es.md)

Ejecuta `LumaRecode.exe` en **PowerShell 7**. Sin argumentos abre la interfaz gráfica; con subcomandos como `probe` y `convert` ejecuta la CLI.

El programa usa el subsistema gráfico de Windows. **Conecta el texto a `Out-Host` o el JSON a `ConvertFrom-Json`** para esperar a que termine y consultar `$LASTEXITCODE`. Otros programas pueden esperar mediante una API de procesos.

La salida usa UTF-8. Estos ejemplos configuran la codificación de la sesión actual de PowerShell.

## Inicio rápido

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --lang es --help | Out-Host
./LumaRecode.exe --version | Out-Host
./LumaRecode.exe license --text | Out-Host
./LumaRecode.exe license --accept-terms | Out-Host
./LumaRecode.exe doctor --json | ConvertFrom-Json
./LumaRecode.exe probe "D:\Videos\clip.mp4" --json | ConvertFrom-Json
./LumaRecode.exe convert "D:\Videos\clip.mp4" | Out-Host
```

La interfaz y la CLI comparten la aceptación de las condiciones de los componentes. Antes del primer uso por lotes, consulta `license --text` y ejecuta `license --accept-terms`, o añade `--accept-terms` a la tarea. `--no-settings` ignora los ajustes guardados de la interfaz y desactiva la escritura de ajustes; añade `--accept-terms` al usar este modo.

Las herramientas se buscan primero en `PATH` y después en las rutas guardadas por la interfaz. Puedes indicar `--ffmpeg PATH --ffprobe PATH`. `--lang system|en|zh|ja|es` selecciona el idioma de salida.

Las fuentes HDR dinámicas usan HEVC por software para conservar metadatos. Usa `--dynamic-hdr standard --purpose share` para HDR estándar o `--range sdr` para SDR. ProRes RAW se revela automáticamente a HLG; usa `--range sdr` para SDR. Consulta [Conversión de color (chino e inglés)](../color-conversion.md#english).

## Comandos

| Comando | Función |
| --- | --- |
| `probe INPUT` | Formatos de vídeo y audio, profundidad, identificación de color y Log, flujos, etiquetas y datos adicionales del primer fotograma |
| `presets` | Preajustes de codificación y parámetros de los codificadores |
| `profiles` | Perfiles y LUT incluidos de DJI, Samsung, Xiaomi y Apple Log |
| `frames INPUT` | Número de fotogramas, duración y base de tiempo |
| `image INPUT` | Exportar una imagen HDR / SDR o Motion Photo |
| `doctor` | Versiones de herramientas y listas de codificadores y filtros; los modos de hardware dependen de la GPU y del controlador |
| `cache [--purge]` | Inventario y plan de retención; `--purge` solo borra dentro de la raíz de caché del programa |
| `license [--text]` | Licencia MIT, condiciones de los componentes, directorio de avisos y manifiesto |
| `plan INPUT...` | Vista previa de las opciones, argumentos de FFmpeg, directorio de trabajo y color de salida |
| `convert INPUT...` | Convierte secuencialmente archivos, un directorio o coincidencias de comodines y verifica la salida |

Todos los comandos admiten `--help`. Los resultados de análisis, catálogos, diagnóstico, caché y licencias son JSON. El progreso en texto se escribe en stderr; los resultados, en stdout.

## Opciones de codificación y color

`convert` y `plan` ofrecen los mismos controles de codificación y color que la interfaz:

| Opción | Valores |
| --- | --- |
| `--preset` | `share-avc`, `share-hevc`, `share-av1`, `archive-avc`, `archive-hevc`, `archive-av1`, `archive-av1-fast` |
| `--purpose` | `share` (Intel QSV), `archive` (CPU); x64 usa compartir, ARM64 usa archivo; las fuentes HDR dinámicas usan archivo por defecto |
| `--codec` | `hevc` (predeterminado), `av1`, `avc` |
| `--range` / `--dynamic-range` | `auto` (predeterminado), `sdr`, `hdr` |
| `--transform` | `auto` (predeterminado), `none`, `log-hdr`, `log-sdr`, `hdr-sdr` |
| `--profile` | `auto`, `pocket3`, `samsung`, `milog`, `applelog`, `applelog2` |
| `--dynamic-hdr` | `preserve` / `standard` |
| `--input-kind` | `auto`, `sdr`, `hlg`, `pq`, `log`, `raw` |
| `--sampling` | `auto` (predeterminado), `source`, `420`, `422`, `444`, `gray`. Auto conserva el submuestreo en archivo HEVC / AVC por software, convierte RGB a 4:4:4 y usa 4:2:0 para AV1 / compartir; 422 / 444 / gray usan HEVC / AVC por software |
| `--color-range` | `auto` (predeterminado), `source`, `limited`, `full`. Auto conserva el rango en archivo y convierte a Limited al compartir |
| `--bit-depth` | `auto`, `8`, `10`, `12`; archivo sigue la profundidad original y los límites del codificador; compartir usa SDR de 8 bits / HDR de 10 bits |
| `--container` | `auto` (MP4 para compartir, MKV para archivo), `mp4`, `mkv` |
| `--audio` | `auto`, `aac`, `opus`, `copy`, `flac`; los valores predeterminados coinciden con la interfaz |
| `--variant` | `standard`, `fast` (solo AV1 por software) |
| `--cpu-decode` / `--no-cpu-decode` | Forzar la decodificación por software / seleccionar automáticamente la decodificación |
| `--duration SECONDS` | Codificar una muestra de duración positiva y finita, en segundos |
| `--config FILE.json` | Cargar opciones de codificación; los campos explícitos de la CLI tienen prioridad |

El modo Auto selecciona SDR → SDR, HLG / PQ → HDR y Log → HLG. Usa `--profile` para elegir la curva Log y `--input-kind` para indicar el tipo de entrada. La salida HDR usa HEVC / AV1.

El submuestreo y la profundidad se comprueban frente al original. Una entrada de 8 bits 4:2:0 permanece en ese formato; una de 10 bits 4:2:2 permite 8 / 10 bits y 4:2:0 / 4:2:2. HEVC por software admite 8 / 10 / 12 bits; AVC por software y AV1 admiten 8 / 10 bits. `--sampling source` convierte RGB a YCbCr 4:4:4. Las entradas en escala de grises pueden usar `source` o `gray`.

La conversión de rango procesa los valores de los píxeles. Si faltan etiquetas de rango, el formato de píxel y el perfil de cámara determinan cómo se interpreta la entrada.

`--audio flac` comprueba todas las pistas y acepta audio entero confirmado como sin pérdidas, conservando la frecuencia de muestreo, los canales y la precisión entera. El audio FLAC usa MKV.

```powershell
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
./LumaRecode.exe convert hdr.mp4 --range sdr | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range sdr | Out-Host
./LumaRecode.exe convert source-422.mov --preset archive-hevc --sampling 422 --audio copy | Out-Host
./LumaRecode.exe plan clip.mp4 --preset archive-av1-fast --json | ConvertFrom-Json
./LumaRecode.exe convert lossless.mov --preset archive-hevc --sampling source --color-range source --audio flac | Out-Host
```

HDR / Log a SDR requiere libplacebo de FFmpeg y Vulkan. Un archivo JSON admite los campos de codificación anteriores con claves separadas por guiones bajos, además de `preset`:

```json
{"preset":"archive-hevc","range":"auto","sampling":"420","bit_depth":8,"audio":"copy","cpu_decode":true}
```

Prioridad: valores iniciales equivalentes a la interfaz → preajuste → campos del archivo de configuración → campos explícitos de la CLI. `--preset` de la CLI tiene prioridad sobre el preajuste del archivo; el resto de sus campos sigue aplicándose. Las claves usan guiones bajos, como `color_range`. `range` indica el rango dinámico; la ruta de salida se especifica con `--output`.

## Intervalos e imágenes

`frames INPUT` devuelve el número de fotogramas, la duración y la base de tiempo. Los índices de la CLI empiezan en 0 y excluyen el fotograma final.

```powershell
./LumaRecode.exe frames clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --start-frame 30 --end-frame 90 --audio aac | Out-Host
./LumaRecode.exe convert clip.mp4 --start-time 1.5 --end-time 4 --audio aac | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr -o photo.jpg | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr --motion --motion-start-frame 30 --motion-end-frame 90 -o photo_MP.jpg | Out-Host
```

Los intervalos por fotogramas, por segundos y `--duration` son excluyentes. Los segundos se ajustan al primer fotograma situado en esa posición o después. El recorte recodifica el audio. El audio predeterminado de Archivo elige FLAC o AAC; un `--audio copy` explícito debe cambiarse por AAC, Opus o FLAC compatible.

`image` acepta una entrada:

| Opción | Significado |
| --- | --- |
| `--frame N` | Fotograma elegido, predeterminado 0 |
| `--image-format auto\|hdr\|ultrahdr\|sdr\|png` | HDR AVIF, Ultra HDR JPEG, JPEG SDR o PNG SDR; auto elige Ultra HDR para HDR y JPEG para SDR |
| `--quality 1..100` | Calidad, predeterminada 95; PNG usa compresión sin pérdida |
| `--motion` | Añadir vídeo de foto en movimiento de Android a JPEG o Ultra HDR |
| `--motion-start-frame N --motion-end-frame N` | Intervalo que incluye la portada; unos tres segundos alrededor de ella de forma predeterminada |
| `--motion-size source\|1080\|720` | Resolución del vídeo; original de forma predeterminada |
| `--purpose archive\|share` | Codificación CPU / Intel del vídeo, predeterminada `archive` |
| `--input-kind`, `--profile` | La misma interpretación de entrada y perfiles de cámara que para vídeo |
| `-o PATH`, `--overwrite`, `--skip-existing` | Destino y gestión de archivos existentes |
| `--dry-run` | Mostrar la selección de imagen y el intervalo de vídeo |

Las fotos en movimiento usan un único `MP.jpg`. Las imágenes HDR requieren vídeo HDR, Log o RAW. Consulta los formatos y las especificaciones de Google en el [README](../readme/es.md).

## Lotes y rutas de salida

```powershell
./LumaRecode.exe convert a.mp4 b.mov --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos" --recursive --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos\**\*.mp4" --recursive --output-dir "D:\Exports" --continue-on-error --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --output "D:\Exports\result.mp4" --overwrite | Out-Host
```

Pon los patrones entre comillas; `**` requiere `--recursive`. La búsqueda en directorios filtra extensiones de vídeo comunes y conserva los subdirectorios relativos. Los comodines toman como raíz el directorio padre común de las coincidencias; los archivos explícitos usan su nombre. Los destinos duplicados se rechazan antes de convertir. Los directorios de salida se crean cuando hacen falta.

Los nombres predeterminados son `stem_SDR-or-HDR_codec_purpose.mp4-or-mkv`, junto al original. Usa `--output` para un archivo o `--output-dir` para un lote; son opciones excluyentes. En modo interactivo se pregunta antes de reemplazar una salida existente y la opción predeterminada es cancelar. Los scripts pueden usar `--skip-existing` para omitirla o `--overwrite` para autorizar el reemplazo directamente. La codificación se realiza en una salida temporal que se verifica antes del reemplazo. Las rutas de origen y destino deben ser distintas. Los fallos y cancelaciones eliminan los archivos temporales.

Las tareas se ejecutan secuencialmente y se detienen ante un error por defecto. `--continue-on-error` procesa el resto de las entradas y refleja los fallos en el código de salida del lote.

El campo `argv` de la planificación muestra la ruta final. La conversión escribe en un directorio temporal, verifica el resultado y lo mueve al destino. `--overwrite` usa un reemplazo atómico.

## Automatización y progreso

`convert --json` / `plan --json` producen JSON Lines en stdout con `schema_version: 1`. Eventos: `start`, `log`, `status`, `decoder`, `progress`, `done`, `skipped`, `error`, `cancelled`, `summary`; la planificación emite `plan`. Los registros de tarea incluyen `job`, `total_jobs`, `input`, `output` y el campo `data` del evento.

`decoder.data`: `software` / `qsv`.

El progreso incluye `fps`, `percent`, `position_seconds`, `duration_seconds`, `elapsed_seconds` y `eta_seconds`; los porcentajes y tiempos restantes desconocidos son null. Los errores incluyen un `code` estable y un `message` legible. `--log FILE` añade los mismos eventos a un archivo JSON Lines; `--verbose` muestra los registros de FFmpeg en modo texto.

```powershell
./LumaRecode.exe convert clip.mp4 --json --log job.jsonl | ForEach-Object { $_ | ConvertFrom-Json }
$LASTEXITCODE
```

| Código | Significado |
| --- | --- |
| 0 | Éxito, incluidas las omisiones solicitadas |
| 1 | Fallo de análisis, codificación o verificación, o fallo parcial del lote |
| 2 | Argumentos, configuración o combinación de opciones no válidos |
| 3 | Entorno de ejecución o herramienta FFmpeg ausente o no válida |
| 4 | Entrada ausente, sin coincidencias, o tipo de entrada / curva Log ambiguos |
| 5 | Conflicto o error en la ruta de salida o del registro |
| 6 | Condiciones de los componentes no aceptadas |
| 130 | Cancelación cooperativa de la CLI |

Consulta el código de salida del proceso o `$LASTEXITCODE` para conocer el resultado, y los campos JSON `code` para gestionar errores.

Ctrl+C durante una ejecución interactiva interrumpe tanto la canalización de PowerShell como la conversión y elimina la salida temporal. Para recoger los eventos de cancelación y el código de salida, envía una señal de cancelación de consola mediante una API de procesos y espera a que termine. La cancelación cooperativa devuelve 130.
