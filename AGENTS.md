# AGENTS.md

Guía para agentes de código (Claude Code, Codex, etc.) que trabajan en este repositorio.

## Qué es este proyecto

Cliente de API nativo y local (macOS, Windows, Linux) escrito en **Rust** con **GPUI** y `gpui-kit = "0.7.1"`. Guarda colecciones como **OpenCollection YAML** (diseñadas para vivir en Git), soporta HTTP y GraphQL, y tiene un **visualizador de JSON en diagrama de nodos**.

## Comandos

```bash
cargo run --release                                   # ejecutar la app
cargo build                                           # compilar
cargo test --workspace                                # pruebas
cargo fmt --all                                       # formato
cargo clippy --workspace --all-targets -- -D warnings # lint
```

Antes de dar una tarea por terminada deben pasar `fmt`, `clippy` y `test`.

## Arquitectura

Workspace de Cargo con dependencias en una sola dirección:

```text
app → ui → core ← storage / importers / http / json-graph
```

- **`core`**: modelos de dominio (`Collection`, `Folder`, `Request`, `Environment`, `Auth`, `Body`). Sin dependencias de UI, red ni disco.
- **`storage`**: lectura/escritura de OpenCollection YAML. Es la única capa que toca el formato en disco.
- **`importers`**: Postman y Yaak → modelos de `core`.
- **`http`**: ejecución de solicitudes HTTP/GraphQL y métricas (estado, tiempo, tamaño).
- **`json-graph`**: JSON → grafo → layout → primitivas de dibujo. Sin dependencia de GPUI en el cálculo; la UI solo dibuja el resultado.
- **`ui`**: vistas GPUI y componentes.
- **`app`**: arranque, ventana y wiring.

Reglas:

- `core` nunca depende de `ui`, `http` ni `storage`.
- La lógica de negocio va fuera de las vistas; las vistas solo reaccionan al estado y emiten acciones.
- Define puertos como traits en `core` y las implementaciones en los demás crates.

## Convenciones de GPUI

- Antes de usar una API de `gpui` o `gpui-kit`, **revisa su documentación o código fuente** en la versión fijada; la API cambia entre versiones. No inventes métodos.
- No fijes ni actualices la versión de `gpui-kit` ni de `gpui` sin que se pida: deben ser mutuamente compatibles.
- Estado en entidades (`Entity<T>`); notifica con `cx.notify()` solo cuando cambie algo visible.
- Las operaciones largas (red, disco, parseo grande) van en tareas asíncronas, **nunca** en el hilo de render.
- Listas largas (sidebar, headers, nodos del grafo) usan listas virtualizadas.
- Acciones y atajos de teclado se declaran con el sistema de acciones de GPUI, no con manejadores ad hoc.
- Componentes pequeños y reutilizables; prefiere los de `gpui-kit` antes de crear uno propio.

## Formato de colecciones (OpenCollection YAML)

- Se respeta el formato OpenCollection: una colección abierta y guardada sin cambios **no debe producir diff** en Git.
- Preserva campos desconocidos al leer/escribir para no perder datos de otras herramientas.
- Orden de claves estable y salida determinista.
- Nunca guardes secretos en texto plano dentro de la colección; los valores sensibles de entorno van en un archivo local ignorado por Git.
- Cambios en el esquema de lectura/escritura requieren pruebas con colecciones de `examples/`.

## Visualizador de JSON (`json-graph`)

Debe reproducir este modelo visual:

- Cada **objeto** es un nodo que lista sus valores primitivos como `clave: valor`, coloreados por tipo (string, número, booleano, null).
- Cada **array u objeto anidado** genera un nodo etiquetado con su clave (`members`, `powers`) con botón de colapsar/expandir, conectado con aristas a sus hijos.
- Los elementos primitivos de un array son nodos hoja.
- Layout de árbol de **izquierda a derecha**; aristas curvas.
- Pan, zoom, colapsar/expandir, selección sincronizada con el editor de JSON.
- Exportación a PNG y SVG.

Requisitos técnicos:

- Pipeline: `serde_json::Value` → `Graph` (nodos + aristas) → `Layout` (posiciones) → dibujo. Cada etapa es pura y testeable.
- Dibuja solo los nodos dentro del viewport; debe funcionar con respuestas de varios MB sin congelar la UI.
- Si el JSON es inválido, muestra el error y no el diagrama; nunca hagas `panic`.
- Pruebas unitarias del grafo y del layout con el JSON de ejemplo (`squadName`, `members`, `powers`).

## Inteligencia de respuesta

- Detecta JWT en la respuesta y muestra header y payload decodificados. **No verifiques la firma ni envíes el token a ningún servicio externo.**
- Detecta timestamps (segundos o milisegundos Unix, ISO 8601) y muéstralos en hora legible.
- Estas detecciones son heurísticas: nunca modifican el cuerpo original de la respuesta.

## Estilo de código

- Rust edición 2021 o superior; `rustfmt` por defecto.
- Errores: `thiserror` en librerías, `anyhow` solo en `app`. Sin `unwrap()`/`expect()` en código de producción salvo invariantes documentadas.
- Nombres de tipos y funciones en inglés; textos visibles al usuario en español, centralizados para poder localizarlos después.
- Comentarios solo para explicar el *porqué*, no el *qué*.
- Evita `unsafe`; si es imprescindible, documenta la razón.

## Pruebas

- Unitarias junto al código (`#[cfg(test)]`); de integración en `tests/`.
- Obligatorias para: lectura/escritura YAML (roundtrip sin diff), importadores, resolución de variables y herencia de entornos, `json-graph` y detección de JWT/timestamps.
- Las pruebas de red usan un servidor local falso, nunca internet real.

## Seguridad y privacidad

- Todo es local: sin telemetría ni envío de datos a terceros.
- No registres en logs headers de autenticación, tokens ni bodies completos.
- Las solicitudes solo salen cuando el usuario las envía explícitamente.

## Flujo de trabajo para agentes

1. Lee este archivo y el código del crate que vas a tocar.
2. Haz cambios pequeños y enfocados; no refactorices código no relacionado.
3. Añade o actualiza pruebas junto al cambio.
4. Ejecuta `fmt`, `clippy` y `test`.
5. Si algo es ambiguo (formato OpenCollection, API de GPUI), consulta la fuente o pregunta; no supongas.
6. No agregues dependencias nuevas sin justificarlas en el mensaje del cambio.