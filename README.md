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
| macOS | Command Line Tools (los shaders de Metal se compilan en tiempo de ejecución; no hace falta Xcode completo) |
| Windows | Visual Studio Build Tools (C++) |
| Linux | Vulkan, Wayland/X11, `libxkbcommon`, `fontconfig` |

> Ely GPUI Component está probado únicamente en macOS. En Windows y Linux, verifica el comportamiento de los componentes antes de depender de ellos.

## Dependencias principales

Stack de UI fijado:

```toml
[dependencies]
# GPUI y stack de UI (Ely GPUI Component + Zed GPUI)
ely-gpui-component = { git = "https://github.com/ZacharyZhang-NY/Ely-GPUI-Components" }
gpui = { git = "https://github.com/zed-industries/zed", rev = "1a28cff4b409169bac058bca40dfbfeb7621d19b" }
gpui_platform = { git = "https://github.com/zed-industries/zed", rev = "1a28cff4b409169bac058bca40dfbfeb7621d19b", features = ["font-kit", "wayland", "x11"] }
```

- **[Ely GPUI Component](https://github.com/ZacharyZhang-NY/Ely-GPUI-Components)**: biblioteca de componentes para GPUI, con tema claro y oscuro de paleta cálida y sobria. Es la base visual de la interfaz.
- **gpui_platform** con `font-kit`, `wayland` y `x11`: soporte multiplataforma para fuentes y servidores de ventanas en macOS/Linux.

Se usarán además `tokio`, `reqwest`, `serde`, `serde_json`, `serde_yaml` y `anyhow`.

### Inicialización

```rust
use ely_gpui_component::{Assets, theme::{Mode, Theme}};
use gpui::App;

fn main() {
    gpui_platform::application().with_assets(Assets).run(|cx: &mut App| {
        ely_gpui_component::init(cx);
        Theme::set_mode(Mode::Dark, cx);
        // ...abrir ventana principal
    });
}
```

`ely_gpui_component::init` registra fuentes y tema, y entra en pánico si `Assets` no está conectado a la aplicación.

### Galería de componentes

Todos los componentes de Ely se pueden ver funcionando en <https://ely-gpui.zacharyzhang.com>. Consúltala antes de crear un componente propio.

## Estructura del proyecto

```text
kestrel/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── app/            # binario, ventana y arranque de GPUI
│   ├── ui/             # vistas y componentes (sidebar, tabs, editor, respuesta)
│   │   └── theme/      # adaptador único de tema (Ely GPUI Components)
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
- [x] Cliente HTTP y GraphQL
- [x] Panel de respuesta (Pretty / Raw / Headers)
- [x] Inteligencia de respuesta (JWT, timestamps)
- [ ] Importadores Postman y Yaak
- [ ] Visualizador de JSON en diagrama
- [ ] Exportar diagrama a PNG/SVG

## Contribuir

1. Lee `AGENTS.md` para convenciones y arquitectura.
2. Antes de abrir un PR: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`.

## Licencia

Por definir (MIT o Apache-2.0).