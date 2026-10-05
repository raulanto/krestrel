# Kestrel

Cliente de API **rápido, nativo y local** para macOS, Windows y Linux. Construido con **Rust** y **GPUI**. Las colecciones se guardan como **OpenCollection YAML** y viven en Git junto a tu código.

> `Kestrel` es un nombre provisional; cámbialo por el definitivo.

## Características

### Colecciones que viven en Git

- Los espacios de trabajo se almacenan como **OpenCollection YAML**: legible, con diffs limpios y natural dentro de un repositorio.
- Abre colecciones OpenCollection existentes tal cual.
- Importa colecciones de **Postman** y **Yaak**.
- Solicitudes organizadas en carpetas en la barra lateral, con búsqueda por nombre y navegación instantánea incluso en colecciones muy grandes.

### Construir y enviar solicitudes

- Cada solicitud se abre en su propia pestaña: método, URL, parámetros de ruta y query, headers, body y autenticación en un editor compacto.
- Bodies: JSON, texto, XML, SPARQL, form, multipart y subida de archivos, con resaltado de sintaxis en formatos estructurados.
- Soporte para **HTTP** y **GraphQL**.
- Entornos (desarrollo, producción…) con variables como `base_url`, herencia, overrides y cambio rápido desde la barra de herramientas.

### Leer respuestas más rápido

- Estado, tiempo y tamaño de un vistazo.
- Vistas **Pretty**, **Raw** y **Headers**.
- **Inteligencia de respuesta**: detecta y destaca detalles útiles automáticamente, como decodificar JWT y traducir timestamps, sin herramientas extra.

### ✨ Visualizador de JSON en diagrama (extra)

Además de la vista Pretty, cualquier respuesta JSON puede verse como un **diagrama de nodos**:

- Cada objeto es un nodo con sus valores primitivos (`clave: valor`) coloreados por tipo.
- Los arrays y objetos anidados se conectan con aristas desde un nodo etiquetado con su clave (`members`, `powers`…), y cada uno se puede **colapsar/expandir**.
- Layout de árbol de izquierda a derecha, con **pan y zoom** sobre el lienzo.
- Vista dividida: JSON a la izquierda, diagrama a la derecha, con selección sincronizada.
- Exportar el diagrama como **PNG/SVG**.
- Funciona con respuestas grandes (renderizado solo de los nodos visibles).

## Instalación

Requisitos: Rust estable (ver `rust-toolchain.toml`) y las dependencias de plataforma de GPUI.

```bash
git clone https://github.com/<tu-usuario>/kestrel.git
cd kestrel
cargo run --release
```

Dependencias de sistema de GPUI:

| SO | Requisitos |
| --- | --- |
| macOS | Xcode y Command Line Tools (Metal) |
| Windows | Visual Studio Build Tools (C++) |
| Linux | Vulkan, Wayland/X11, `libxkbcommon`, `fontconfig` |

## Dependencias principales

```toml
[dependencies]
gpui-kit = "0.7.1"
```

Se usarán además `gpui`, `tokio`, `reqwest`, `serde`, `serde_json`, `serde_yaml` y `anyhow`. Revisa que la versión de `gpui` sea la compatible con `gpui-kit 0.7.1`.

## Estructura del proyecto

```text
kestrel/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── app/            # binario, ventana y arranque de GPUI
│   ├── ui/             # vistas y componentes (sidebar, tabs, editor, respuesta)
│   ├── core/           # modelos de dominio: Collection, Request, Environment
│   ├── storage/        # lectura/escritura OpenCollection YAML
│   ├── importers/      # Postman y Yaak
│   ├── http/           # cliente HTTP y GraphQL
│   └── json-graph/     # parseo JSON → grafo, layout y exportación
└── examples/           # colecciones de ejemplo
```

## Hoja de ruta

- [x] Abrir y guardar colecciones OpenCollection YAML
- [x] Sidebar con carpetas y búsqueda
- [x] Pestañas de solicitud y editor de body
- [x] Entornos y variables
- [ ] Cliente HTTP y GraphQL
- [ ] Panel de respuesta (Pretty / Raw / Headers)
- [ ] Inteligencia de respuesta (JWT, timestamps)
- [ ] Importadores Postman y Yaak
- [ ] Visualizador de JSON en diagrama
- [ ] Exportar diagrama a PNG/SVG

## Contribuir

1. Lee `AGENTS.md` para convenciones y arquitectura.
2. Antes de abrir un PR: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`.

## Licencia

Por definir (MIT o Apache-2.0).