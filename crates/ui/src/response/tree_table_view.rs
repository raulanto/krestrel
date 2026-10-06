use ely_gpui_component::data_display::Tone;
use ely_gpui_component::primitives::{Icon, IconName};
use ely_gpui_component::tables::{Cell, Column, TreeRow, TreeTable};
use gpui::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, rems,
};
use serde_json::Value;

use crate::theme::{IconSize, ThemeExt as _, v_flex};

#[derive(IntoElement)]
pub struct TreeTableView {
    root_value: Option<Value>,
    error_message: Option<String>,
}

impl TreeTableView {
    pub fn new(body_str: &str) -> Self {
        if body_str.trim().is_empty() {
            return Self {
                root_value: None,
                error_message: Some("El cuerpo de la respuesta está vacío".into()),
            };
        }

        match serde_json::from_str::<Value>(body_str) {
            Ok(val) => Self {
                root_value: Some(val),
                error_message: None,
            },
            Err(err) => Self {
                root_value: None,
                error_message: Some(format!("No es un JSON válido para TreeTable: {}", err)),
            },
        }
    }
}

/// Converts a `serde_json::Value` into hierarchical `TreeRow`s for `TreeTable`.
pub fn value_to_tree_rows(
    val: &Value,
    parent_path: &str,
    open_keys: &mut Vec<SharedString>,
) -> Vec<TreeRow> {
    match val {
        Value::Object(map) => {
            let mut rows = Vec::new();
            for (k, v) in map {
                let path = if parent_path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", parent_path, k)
                };

                let (type_cell, val_cell, has_children) = describe_value(v);
                let mut row = TreeRow::new(
                    path.clone(),
                    vec![Cell::Text(k.clone().into()), type_cell, val_cell],
                );

                if has_children {
                    if parent_path.is_empty() || !parent_path.contains('.') {
                        open_keys.push(path.clone().into());
                    }
                    let child_rows = value_to_tree_rows(v, &path, open_keys);
                    row = row.children(child_rows);
                }

                rows.push(row);
            }
            rows
        }
        Value::Array(arr) => {
            let mut rows = Vec::new();
            for (idx, v) in arr.iter().enumerate() {
                let path = if parent_path.is_empty() {
                    format!("[{}]", idx)
                } else {
                    format!("{}[{}]", parent_path, idx)
                };

                let (type_cell, val_cell, has_children) = describe_value(v);
                let mut row = TreeRow::new(
                    path.clone(),
                    vec![Cell::Text(format!("[{}]", idx).into()), type_cell, val_cell],
                );

                if has_children {
                    if parent_path.is_empty() || !parent_path.contains('.') {
                        open_keys.push(path.clone().into());
                    }
                    let child_rows = value_to_tree_rows(v, &path, open_keys);
                    row = row.children(child_rows);
                }

                rows.push(row);
            }
            rows
        }
        _ => {
            let (type_cell, val_cell, _) = describe_value(val);
            let path = if parent_path.is_empty() {
                "root".to_string()
            } else {
                parent_path.to_string()
            };
            vec![TreeRow::new(
                path,
                vec![Cell::Text("root".into()), type_cell, val_cell],
            )]
        }
    }
}

fn describe_value(v: &Value) -> (Cell, Cell, bool) {
    match v {
        Value::Null => (
            Cell::Tag("null".into(), Tone::Neutral),
            Cell::Text("null".into()),
            false,
        ),
        Value::Bool(b) => (
            Cell::Tag("bool".into(), Tone::Info),
            Cell::Text(b.to_string().into()),
            false,
        ),
        Value::Number(n) => (
            Cell::Tag("number".into(), Tone::Success),
            Cell::Text(n.to_string().into()),
            false,
        ),
        Value::String(s) => {
            let display = if s.len() > 100 {
                format!("\"{}...\"", &s[..97])
            } else {
                format!("\"{}\"", s)
            };
            (
                Cell::Tag("string".into(), Tone::Neutral),
                Cell::Text(display.into()),
                false,
            )
        }
        Value::Array(arr) => (
            Cell::Tag(format!("array [{}]", arr.len()).into(), Tone::Warning),
            Cell::Text(format!("{} elementos", arr.len()).into()),
            !arr.is_empty(),
        ),
        Value::Object(obj) => (
            Cell::Tag(format!("object {{{}}}", obj.len()).into(), Tone::Accent),
            Cell::Text(format!("{} campos", obj.len()).into()),
            !obj.is_empty(),
        ),
    }
}

impl RenderOnce for TreeTableView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        if let Some(err) = self.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .p_8()
                .child(
                    Icon::new(IconName::FileText)
                        .size(IconSize::Lg)
                        .color(colors.fg_muted),
                )
                .child(div().text_sm().text_color(colors.fg_muted).child(err))
                .into_any_element();
        }

        let Some(ref val) = self.root_value else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_8()
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.fg_muted)
                        .child("Sin contenido para mostrar en tabla"),
                )
                .into_any_element();
        };

        let mut open_keys = Vec::new();
        let rows = value_to_tree_rows(val, "", &mut open_keys);

        let columns = vec![
            Column::new("key", "Propiedad / Clave").width(rems(16.)),
            Column::new("type", "Tipo").width(rems(9.)),
            Column::new("value", "Valor"),
        ];

        let table = TreeTable::new("response-tree-table", columns, rows).open(open_keys);

        v_flex()
            .id("response-tree-table-view")
            .size_full()
            .overflow_y_scroll()
            .p_2()
            .bg(colors.bg)
            .child(table)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_to_tree_rows_flat_object() {
        let json: Value = serde_json::json!({
            "name": "Kestrel",
            "active": true,
            "version": 1
        });
        let mut open = Vec::new();
        let rows = value_to_tree_rows(&json, "", &mut open);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_value_to_tree_rows_nested_structure() {
        let json: Value = serde_json::json!({
            "user": {
                "name": "Alice",
                "roles": ["admin", "editor"]
            }
        });
        let mut open = Vec::new();
        let rows = value_to_tree_rows(&json, "", &mut open);
        assert_eq!(rows.len(), 1);
        assert!(open.contains(&"user".into()));
    }

    #[test]
    fn test_tree_table_view_invalid_json() {
        let view = TreeTableView::new("not a json string");
        assert!(view.root_value.is_none());
        assert!(view.error_message.is_some());
    }

    #[test]
    fn test_tree_table_view_valid_json() {
        let view = TreeTableView::new(r#"{"status": "ok"}"#);
        assert!(view.root_value.is_some());
        assert!(view.error_message.is_none());
    }
}
