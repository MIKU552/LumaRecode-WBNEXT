<p align="center">
  <img src="../../Brand/LumaRecode.svg" width="104" height="104" alt="Icono de LumaRecode" />
</p>
<h1 align="center">映码 · LumaRecode WBNEXT</h1>

[简体中文](../../README.md) | [English](en.md) | [日本語](ja.md) | Español

> **Vista previa de la reescritura WBNEXT**: esta rama es una reescritura multiplataforma independiente basada en [LumaRecode 4.4.3](https://github.com/MIKU552/LumaRecode), desarrollada con WorkBuddy (DeepSeek-V4.1-Flash-1017, razonamiento alto, contexto de 1M). **No es una versión oficial** y no hereda el soporte del proyecto original. Las compilaciones provienen de los artefactos de GitHub Actions; el avance se registra en el [registro de la reescritura](../refactor/README.md#english).

Herramienta de transcodificación de vídeo y conversión de color para Windows 11, desarrollada con C# y .NET. LumaRecode ofrece una interfaz nativa WinUI 3 y una línea de comandos para FFmpeg, con compresión de vídeo, conversión de Log de cámara a HDR / SDR y de HDR a SDR.

[Descargar](https://github.com/MIKU552/LumaRecode/releases/latest) · [Documentación (chino e inglés)](../README.md#english) · [Línea de comandos](../cli/es.md) · [Historial de cambios (chino e inglés)](../../CHANGELOG.md#english)

## Funciones

- Vista previa y recorte por fotogramas, imágenes HDR / SDR y fotos en movimiento de Android con portada seleccionada.
- Preajustes de Compartir rápido y Archivo por software para HEVC, AV1 y H.264, con salida MP4 / MKV.
- Conversión de DJI D-Log M, Samsung Log, Mi-Log, Apple Log y Apple Log 2 a HLG HDR o Rec.709 SDR; conversión de HLG / PQ a SDR.
- Revelado de ProRes RAW, detección y conversión de Dolby Vision / HDR10+ y conservación de metadatos dinámicos con codificación por software.
- Lectura de resolución, tasa de fotogramas, profundidad de bits, etiquetas de color y pistas de audio, con ajustes iniciales según el material.
- Selección de submuestreo de croma, rango Limited / Full y formato de audio, incluido FLAC sin pérdidas.
- Progreso, fps de codificación, posición, tiempo restante y verificación del formato de salida.
- Notificaciones de Windows al completar o fallar una tarea, con una casilla junto a Abrir carpeta.
- Mica, temas claro / oscuro / del sistema y colores de énfasis. Interfaz en chino simplificado, inglés, japonés y español.
- Procesamiento por lotes, archivos de configuración, vista previa de comandos y eventos JSON mediante la CLI.

## Inicio rápido

1. Descarga el EXE para tu equipo desde [Releases](https://github.com/MIKU552/LumaRecode/releases). Consulta las variantes en la tabla siguiente. Los entornos de ejecución necesarios están incluidos.
2. Prepara una compilación de [FFmpeg para Windows (inglés)](https://ffmpeg.org/download.html) con `ffmpeg.exe` y `ffprobe.exe`. LumaRecode busca en `PATH` al iniciarse; también puedes indicar las rutas con el botón de engranaje.
3. Arrastra un vídeo o pulsa **Elegir vídeo** y comprueba el rango dinámico, el preajuste y la conversión de color.
4. Elige la ruta de salida y pulsa **Iniciar conversión**.

Compartir rápido usa Intel QSV. Archivo por software usa la CPU.

| Archivo | Plataforma y requisitos |
| --- | --- |
| `LumaRecode.exe` | X64, con un perfil de instrucciones x64 común |
| `LumaRecode-ARM64.exe` | Windows ARM64, con aplicación y códec de imagen nativos ARM64 |

ARM64 usa Archivo por software de forma predeterminada. La velocidad de codificación depende del FFmpeg externo, del hardware y del preajuste.

Las fuentes Dolby Vision / HDR10+ usan HEVC por software para conservar los metadatos dinámicos de forma predeterminada. Elige HDR estándar para compartir con codificación por hardware. Se admiten los perfiles Dolby Vision de una sola capa 5, 8.1, 8.4, 10.1 y 10.4. ProRes RAW comprueba la compatibilidad de FFmpeg y se revela con el balance de blancos, la matriz de color y la ganancia de cámara, con salida HLG predeterminada. Consulta los formatos compatibles en [Conversión de color (chino e inglés)](../color-conversion.md#english).

## Vista previa, recorte e imágenes

Abre **Vista previa y recorte por fotogramas** en la página de vídeo para desplazarte por la línea de tiempo, avanzar fotograma a fotograma y elegir el inicio y el final. Se convierte todo el vídeo de forma predeterminada. La interfaz cuenta desde 1 e incluye el último fotograma elegido.

La página **Imágenes y fotos en movimiento** exporta el fotograma seleccionado:

| Formato | Salida |
| --- | --- |
| HDR AVIF | BT.2020 / PQ, 10 o 12 bits |
| Ultra HDR JPEG | Imagen base SDR, mapa de ganancia HDR y sus metadatos |
| SDR JPEG | Imagen sRGB |
| SDR PNG | Imagen sRGB sin pérdida, 8 bits o contenedor de 16 bits |

JPEG y Ultra HDR JPEG pueden incluir vídeo de foto en movimiento de Android. La portada y el intervalo de vídeo se eligen por separado; el intervalo inicial es de unos tres segundos alrededor de la portada. Un único archivo `MP.jpg` sigue [Google Motion Photo 1.0 (inglés)](https://developer.android.com/media/platform/motion-photo-format). La codificación Ultra HDR utiliza [libultrahdr (inglés)](https://github.com/google/libultrahdr) de Google. El vídeo usa la CPU de forma predeterminada y también permite codificación rápida de Intel.

La vista previa se muestra en SDR. La imagen se exporta en el formato elegido. El recorte por fotogramas recodifica el audio como AAC, Opus o FLAC compatible.

El vídeo conserva la resolución original de forma predeterminada, con límites opcionales de 1080p y 720p. Los preajustes independientes para clips cortos usan x264 fast / CRF20, x265 fast / CRF23 o Intel TU6. El audio es AAC-LC a 48 kHz; conserva mono o usa estéreo. Consulta [motion-photo-presets.json](../../Assets/motion-photo-presets.json).

## Codificación y color

| Uso | Codificadores | Contenedor predeterminado | Audio predeterminado |
| --- | --- | --- | --- |
| Compartir rápido | Intel QSV: HEVC / AV1 / H.264 | MP4 | AAC para HEVC / H.264; Opus para AV1 |
| Archivo por software | x265 / SVT-AV1 / x264 | MKV | Copiar las pistas originales |

Puedes elegir por separado el contenedor, el submuestreo, la profundidad de bits, el rango de color y el audio. Compartir rápido usa por defecto SDR de 8 bits o HDR de 10 bits. Archivo por software conserva la profundidad del original dentro de los límites del codificador: 8 / 10 / 12 bits para HEVC y 8 / 10 bits para AVC y AV1.

El archivo con HEVC / H.264 por software conserva el submuestreo original. Las entradas RGB se convierten por defecto a YCbCr 4:4:4; AV1 y Compartir rápido usan 4:2:0. Puedes reducir el submuestreo y la profundidad dentro de los límites del original: una entrada de 8 bits 4:2:0 permanece en ese formato; una de 10 bits 4:2:2 permite elegir 10 / 8 bits y 4:2:2 / 4:2:0. Archivo por software conserva el rango Limited / Full original; Compartir rápido usa Limited. El submuestreo, la profundidad y el rango están en **Conversión de color**; el contenedor y el audio están en **Formato y audio**.

FLAC admite audio entero confirmado como sin pérdidas, incluido PCM entero, y conserva la frecuencia de muestreo, los canales y la precisión entera de cada pista. El audio FLAC usa MKV.

El material SDR se convierte por defecto a SDR; HLG / PQ conserva HDR; los perfiles Log reconocidos se convierten a HLG. **Interpretación** y **Perfil de cámara** permiten indicar manualmente el tipo de material.

El vídeo se vuelve a codificar con el preajuste elegido; el audio puede copiarse o convertirse. Los parámetros están en [recommended-presets.json](../../Assets/recommended-presets.json) y las mediciones en el [informe de pruebas de codificación (chino e inglés)](../benchmarks/encoding-benchmark.md#english).

## Requisitos

- Windows 11 x64 o ARM64, con las instrucciones requeridas por la variante elegida.
- FFmpeg / ffprobe con los codificadores y filtros necesarios, aportados por el usuario.
- Compatibilidad con Intel QSV para Compartir rápido.
- `zscale` y `lut3d` para la conversión de Log; `libplacebo` y Vulkan para HDR / Log a SDR.

El idioma y el tema siguen al sistema de forma predeterminada. Se reconocen sistemas en chino, inglés, japonés y español; los demás usan inglés. Los ajustes se guardan en `%LOCALAPPDATA%\LumaRecode\settings.json`.

En el primer inicio, LumaRecode importa el idioma, la apariencia y las rutas de herramientas de los ajustes de FrameFlow.

Source Sans 3, Source Han Sans y Source Code Pro están incluidos; no necesitas instalar fuentes.

## Línea de comandos

En PowerShell 7, conecta la salida a una canalización:

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --lang es --help | Out-Host
./LumaRecode.exe probe clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
```

Antes del primer uso desatendido, consulta las condiciones de los componentes con `license --text` y ejecuta `license --accept-terms`. La [guía de la CLI](../cli/es.md) incluye todos los comandos, el procesamiento por lotes y los códigos de salida.

## Compilar desde el código fuente

Requiere .NET SDK 11.0.100 y PowerShell 7. Las compilaciones AOT opcionales también necesitan las herramientas C++ de Visual Studio.

```powershell
git clone https://github.com/MIKU552/LumaRecode.git
cd LumaRecode
./build.ps1
./build.ps1 -Target X64
./build.ps1 -Target X64 -Compilation Aot
./build.ps1 -Target Arm64
```

JIT es el modo predeterminado. Native selecciona X64 o ARM64 según la arquitectura de Windows y genera `dist/SelfContained/LumaRecode.exe`. Releases ofrece EXE únicos y autocontenidos para X64 y ARM64.

El JIT selecciona las instrucciones compatibles en el equipo de ejecución. En equipos x64 donde la CPU y Windows admiten APX, el inicio habilita APX y respeta la configuración explícita del usuario. La opción `-Compilation Aot` compila al publicar y genera `dist/Aot-SelfContained[-Target]/`. El destino APX fijo utiliza `-Target Apx -Compilation Aot`.

Abre `LumaRecode.sln` con Visual Studio 2026 18.12 o posterior. Usa `LumaRecode.App` como proyecto de inicio y Debug para depurar. `Release | Native`, `Release | x64` y `Release | ARM64` generan JIT; `Release-AOT` genera los destinos AOT correspondientes y `Release-APX | x64` genera APX AOT. `Packaging` crea el EXE único. Consulta [desarrollo y distribución (chino e inglés)](../dependencies.md#english) y la [guía de QA (chino e inglés)](../../qa/README.md#english).

## Estructura del proyecto

```text
App/       Interfaz WinUI 3, apariencia, idiomas y gestión de tareas
Bundle/    Empaquetado en un único archivo
Packaging/ Compilación y distribución desde Visual Studio
Core/      Análisis del material, reglas de formato, FFmpeg y CLI
Assets/    LUT, preajustes de codificación y registros de origen
Brand/     Archivos originales del icono y herramientas de generación
tools/     Recopilación de licencias y generación de código de idiomas
qa/        Comprobaciones automáticas y medición de memoria
docs/      Guías, documentación de desarrollo y datos de pruebas
licenses/  Licencias, avisos y procedencia de componentes de terceros
```

## Desarrollo y licencia

El código original del proyecto se desarrolló con **Codex para Windows**, usando **[GPT-6.1 Sol (inglés)](https://developers.openai.com/api/docs/models/gpt-6.1-sol)** y el nivel de razonamiento **Extra high (`xhigh`)**. **[MIKU552](https://github.com/MIKU552)** define los requisitos, aporta el equipo y el material, y verifica la aplicación en dispositivos reales.

El código, la documentación, los gráficos originales y las LUT de Log de cámara generadas por el proyecto usan la **[licencia MIT (inglés)](../../LICENSE)**. Las licencias y procedencia de las LUT, fuentes y entornos de ejecución de terceros figuran en [NOTICE (inglés)](../../NOTICE) y en [componentes de terceros (chino e inglés)](../../Assets/THIRD_PARTY.md#english).

## Documentación

- [Línea de comandos](../cli/es.md)
- [Desarrollo y distribución (chino e inglés)](../dependencies.md#english)
- [Rendimiento (chino e inglés)](../performance.md#english)
- [Pruebas de codificación (chino e inglés)](../benchmarks/encoding-benchmark.md#english)
- [Historial de cambios (chino e inglés)](../../CHANGELOG.md#english)
