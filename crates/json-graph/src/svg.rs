use crate::graph::{Graph, NodeKind, PrimitiveType};
use crate::layout::Layout;

#[derive(Debug, Clone)]
pub struct SvgTheme {
    pub bg: String,
    pub card_bg: String,
    pub card_border: String,
    pub card_header_bg: String,
    pub text_primary: String,
    pub text_muted: String,
    pub text_string: String,
    pub text_number: String,
    pub text_bool: String,
    pub text_null: String,
    pub edge_stroke: String,
}

impl Default for SvgTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl SvgTheme {
    pub fn dark() -> Self {
        Self {
            bg: "#111216".into(),
            card_bg: "#1a1b22".into(),
            card_border: "#2d303e".into(),
            card_header_bg: "#22242f".into(),
            text_primary: "#f0f2f8".into(),
            text_muted: "#8c92a4".into(),
            text_string: "#7ee787".into(),
            text_number: "#79c0ff".into(),
            text_bool: "#ffa657".into(),
            text_null: "#ff7b72".into(),
            edge_stroke: "#484f58".into(),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: "#f8f9fa".into(),
            card_bg: "#ffffff".into(),
            card_border: "#d0d7de".into(),
            card_header_bg: "#f3f4f6".into(),
            text_primary: "#1f2328".into(),
            text_muted: "#656d76".into(),
            text_string: "#1a7f37".into(),
            text_number: "#0969da".into(),
            text_bool: "#bc4c00".into(),
            text_null: "#cf222e".into(),
            edge_stroke: "#8c959f".into(),
        }
    }
}

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn generate_svg(layout: &Layout, graph: &Graph, theme: &SvgTheme) -> String {
    let padding = 32.0;
    let width = layout.bounds.width + padding * 2.0;
    let height = layout.bounds.height + padding * 2.0;
    let offset_x = padding - layout.bounds.x;
    let offset_y = padding - layout.bounds.y;

    let mut svg = String::with_capacity(4096);

    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace">
<rect width="100%" height="100%" fill="{bg}"/>
<g transform="translate({offset_x}, {offset_y})">
"#,
        width = width,
        height = height,
        bg = theme.bg,
        offset_x = offset_x,
        offset_y = offset_y,
    ));

    // Draw edges
    for edge in &layout.edges {
        svg.push_str(&format!(
            r#"<path d="M {x1} {y1} C {c1x} {c1y}, {c2x} {c2y}, {x2} {y2}" fill="none" stroke="{stroke}" stroke-width="1.75" stroke-linecap="round"/>
"#,
            x1 = edge.start.x,
            y1 = edge.start.y,
            c1x = edge.control1.x,
            c1y = edge.control1.y,
            c2x = edge.control2.x,
            c2y = edge.control2.y,
            x2 = edge.end.x,
            y2 = edge.end.y,
            stroke = theme.edge_stroke,
        ));
    }

    // Draw nodes
    for (node_id, l_node) in &layout.nodes {
        let Some(g_node) = graph.node(node_id) else {
            continue;
        };

        let x = l_node.bounds.x;
        let y = l_node.bounds.y;
        let w = l_node.bounds.width;
        let h = l_node.bounds.height;

        match g_node.kind {
            NodeKind::ObjectCard => {
                // Card background
                svg.push_str(&format!(
                    r#"<g id="{id}">
<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="{card_bg}" stroke="{card_border}" stroke-width="1"/>
<path d="M {x} {y_header} L {x_right} {y_header}" stroke="{card_border}" stroke-width="1"/>
<text x="{x_pad}" y="{y_title}" fill="{text_primary}" font-size="12" font-weight="600">{label}</text>
"#,
                    id = escape_xml(node_id),
                    x = x,
                    y = y,
                    w = w,
                    h = h,
                    card_bg = theme.card_bg,
                    card_border = theme.card_border,
                    x_right = x + w,
                    y_header = y + 28.0,
                    x_pad = x + 10.0,
                    y_title = y + 18.0,
                    text_primary = theme.text_primary,
                    label = escape_xml(&g_node.label),
                ));

                // Property rows
                let mut current_row_y = y + 46.0;
                for prop in &g_node.properties {
                    let val_color = match prop.value.primitive_type() {
                        PrimitiveType::String => &theme.text_string,
                        PrimitiveType::Number => &theme.text_number,
                        PrimitiveType::Boolean => &theme.text_bool,
                        PrimitiveType::Null => &theme.text_null,
                    };

                    svg.push_str(&format!(
                        r#"<text x="{x_pad}" y="{row_y}" font-size="11"><tspan fill="{text_muted}">{key}: </tspan><tspan fill="{val_color}">{val}</tspan></text>
"#,
                        x_pad = x + 10.0,
                        row_y = current_row_y,
                        text_muted = theme.text_muted,
                        key = escape_xml(&prop.key),
                        val_color = val_color,
                        val = escape_xml(&prop.value.display_text()),
                    ));
                    current_row_y += 20.0;
                }

                if g_node.truncated_rows > 0 {
                    svg.push_str(&format!(
                        r#"<text x="{x_pad}" y="{row_y}" fill="{text_muted}" font-size="10" font-style="italic">+{count} más…</text>
"#,
                        x_pad = x + 10.0,
                        row_y = current_row_y,
                        text_muted = theme.text_muted,
                        count = g_node.truncated_rows,
                    ));
                }

                svg.push_str("</g>\n");
            }
            NodeKind::ArrayContainer { count } => {
                svg.push_str(&format!(
                    r#"<g id="{id}">
<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="{card_bg}" stroke="{card_border}" stroke-width="1"/>
<text x="{x_pad}" y="{y_text}" font-size="11" font-weight="600"><tspan fill="{text_primary}">{label} </tspan><tspan fill="{text_muted}">[{count}]</tspan></text>
</g>
"#,
                    id = escape_xml(node_id),
                    x = x,
                    y = y,
                    w = w,
                    h = h,
                    card_bg = theme.card_bg,
                    card_border = theme.card_border,
                    x_pad = x + 10.0,
                    y_text = y + h / 2.0 + 4.0,
                    text_primary = theme.text_primary,
                    text_muted = theme.text_muted,
                    label = escape_xml(&g_node.label),
                    count = count,
                ));
            }
            NodeKind::ArrayChunk { start, end } => {
                svg.push_str(&format!(
                    r#"<g id="{id}">
<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="{card_header_bg}" stroke="{card_border}" stroke-width="1" stroke-dasharray="3,3"/>
<text x="{x_pad}" y="{y_text}" fill="{text_muted}" font-size="10">[{start} … {end}]</text>
</g>
"#,
                    id = escape_xml(node_id),
                    x = x,
                    y = y,
                    w = w,
                    h = h,
                    card_header_bg = theme.card_header_bg,
                    card_border = theme.card_border,
                    x_pad = x + 10.0,
                    y_text = y + h / 2.0 + 4.0,
                    text_muted = theme.text_muted,
                    start = start,
                    end = end,
                ));
            }
            NodeKind::PrimitiveLeaf => {
                let (val_text, val_color) = if let Some(p) = g_node.properties.first() {
                    let color = match p.value.primitive_type() {
                        PrimitiveType::String => &theme.text_string,
                        PrimitiveType::Number => &theme.text_number,
                        PrimitiveType::Boolean => &theme.text_bool,
                        PrimitiveType::Null => &theme.text_null,
                    };
                    (p.value.display_text(), color)
                } else {
                    (g_node.label.clone(), &theme.text_primary)
                };

                svg.push_str(&format!(
                    r#"<g id="{id}">
<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="{card_bg}" stroke="{card_border}" stroke-width="1"/>
<text x="{x_pad}" y="{y_text}" fill="{val_color}" font-size="11">{val}</text>
</g>
"#,
                    id = escape_xml(node_id),
                    x = x,
                    y = y,
                    w = w,
                    h = h,
                    card_bg = theme.card_bg,
                    card_border = theme.card_border,
                    x_pad = x + 10.0,
                    y_text = y + h / 2.0 + 4.0,
                    val_color = val_color,
                    val = escape_xml(&val_text),
                ));
            }
        }
    }

    svg.push_str("</g>\n</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutOptions, calculate_layout};
    use crate::measure::ApproximateMeasurer;
    use std::collections::HashSet;

    #[test]
    fn test_svg_generation_snapshot() {
        let json_str = r#"{
            "squadName": "Super hero squad",
            "homeTown": "Metro City",
            "members": [
                {
                    "name": "Molecule Man",
                    "powers": ["Radiation resistance", "Turning tiny"]
                }
            ]
        }"#;

        let val = serde_json::from_str(json_str).unwrap();
        let graph = Graph::from_json(&val);
        let layout = calculate_layout(
            &graph,
            &HashSet::new(),
            &ApproximateMeasurer::new(),
            LayoutOptions::default(),
        );

        let svg = generate_svg(&layout, &graph, &SvgTheme::dark());

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("Super hero squad"));
        assert!(svg.contains("Molecule Man"));
        assert!(svg.contains("Radiation resistance"));
    }
}
