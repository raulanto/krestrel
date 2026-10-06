use kestrel_json_graph::{
    ApproximateMeasurer, Graph, LayoutOptions, SvgTheme, calculate_layout, generate_svg,
    render_svg_to_png,
};
use serde_json::Value;
use std::collections::HashSet;

#[test]
fn test_edge_cases_and_arbitrary_json() {
    let test_cases = vec![
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Number(0.into()),
        Value::Number((-999999).into()),
        Value::String("".into()),
        Value::String("Emojis: 🚀 ✨ 💻 \n Multi-line\t\r and \"special\" characters <xml>".into()),
        Value::Array(vec![]),
        Value::Object(serde_json::Map::new()),
        serde_json::json!({
            "empty_arr": [],
            "empty_obj": {},
            "nested_empty": [{}, []],
            "unicode": "áéíóú ñ 中文 🔥"
        }),
    ];

    let measurer = ApproximateMeasurer::new();

    for val in test_cases {
        let graph = Graph::from_json(&val);
        let layout = calculate_layout(&graph, &HashSet::new(), &measurer, LayoutOptions::default());

        let svg_dark = generate_svg(&layout, &graph, &SvgTheme::dark());
        assert!(!svg_dark.is_empty());

        let svg_light = generate_svg(&layout, &graph, &SvgTheme::light());
        assert!(!svg_light.is_empty());

        let png = render_svg_to_png(&svg_dark, 1.0).unwrap();
        assert!(!png.is_empty());
    }
}

#[test]
fn test_deeply_nested_json() {
    let mut val = Value::String("deepest".into());
    for i in 0..150 {
        val = serde_json::json!({ format!("level_{}", i): val });
    }

    let graph = Graph::from_json(&val);
    assert!(graph.has_exceeded_limits || graph.total_nodes > 0);

    let measurer = ApproximateMeasurer::new();
    let layout = calculate_layout(&graph, &HashSet::new(), &measurer, LayoutOptions::default());

    assert!(!layout.nodes.is_empty());
}
