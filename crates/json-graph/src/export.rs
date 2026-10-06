use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Error al parsear el SVG: {0}")]
    SvgParseError(String),
    #[error("Error al renderizar el mapa de píxeles: {0}")]
    RenderError(String),
    #[error("Error al codificar la imagen PNG: {0}")]
    PngEncodeError(String),
}

/// Renders an SVG string to PNG image bytes at the specified resolution scale (e.g. 1.0, 2.0).
pub fn render_svg_to_png(svg_str: &str, scale: f32) -> Result<Vec<u8>, ExportError> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| ExportError::SvgParseError(e.to_string()))?;

    let valid_scale = scale.clamp(0.5, 4.0);
    let size = tree.size();
    let width = (size.width() * valid_scale).ceil() as u32;
    let height = (size.height() * valid_scale).ceil() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1))
        .ok_or_else(|| ExportError::RenderError("No se pudo asignar el búfer de píxeles".into()))?;

    let transform = resvg::tiny_skia::Transform::from_scale(valid_scale, valid_scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| ExportError::PngEncodeError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_svg_to_png() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="red"/>
        </svg>"#;

        let png = render_svg_to_png(svg, 1.0).unwrap();
        assert!(!png.is_empty());
        assert_eq!(&png[1..4], b"PNG");
    }
}
