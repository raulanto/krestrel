use gpui::{FontWeight, IntoElement, ParentElement as _, Styled, div, prelude::FluentBuilder as _};
use kestrel_http::{HighlightRange, SyntaxToken};

use crate::theme::ThemeExt as _;

/// Renders a line of text with syntax highlighting according to theme colors
pub fn render_highlighted_line(
    line: &str,
    line_offset: usize,
    highlights: &[HighlightRange],
    _search_query: Option<&str>,
    is_word_wrap: bool,
    cx: &gpui::App,
) -> gpui::AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let line_len = line.len();
    let line_end = line_offset + line_len;

    if highlights.is_empty() {
        return div()
            .font_family("monospace")
            .text_xs()
            .text_color(colors.fg)
            .when(is_word_wrap, |s| s.flex_wrap())
            .child(line.to_string())
            .into_any_element();
    }

    let mut spans = Vec::new();
    let mut current_idx = 0;

    // Filter highlights that overlap with this line
    let mut line_highlights: Vec<(usize, usize, SyntaxToken)> = highlights
        .iter()
        .filter(|h| h.range.end > line_offset && h.range.start < line_end)
        .map(|h| {
            let start = h.range.start.saturating_sub(line_offset).min(line_len);
            let end = (h.range.end - line_offset).min(line_len);
            (start, end, h.token)
        })
        .collect();

    line_highlights.sort_by_key(|&(s, _, _)| s);

    for (start, end, token) in line_highlights {
        if start > current_idx {
            spans.push(
                div()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(colors.fg)
                    .child(line[current_idx..start].to_string()),
            );
        }

        if start < end && end <= line_len {
            let token_color = match token {
                SyntaxToken::Key => colors.accent,
                SyntaxToken::String => colors.success,
                SyntaxToken::Number => colors.warning,
                SyntaxToken::Boolean => colors.info,
                SyntaxToken::Null => colors.fg_muted,
                SyntaxToken::Punctuation => colors.fg_muted,
                SyntaxToken::Tag | SyntaxToken::AttributeName => colors.accent,
                SyntaxToken::AttributeValue => colors.success,
                SyntaxToken::Comment => colors.fg_muted,
                SyntaxToken::Plain => colors.fg,
            };

            let is_bold = matches!(
                token,
                SyntaxToken::Key | SyntaxToken::Boolean | SyntaxToken::Null
            );

            let mut span = div()
                .font_family("monospace")
                .text_xs()
                .text_color(token_color)
                .child(line[start..end].to_string());
            if is_bold {
                span = span.font_weight(FontWeight::SEMIBOLD);
            }
            spans.push(span);
            current_idx = end;
        }
    }

    if current_idx < line_len {
        spans.push(
            div()
                .font_family("monospace")
                .text_xs()
                .text_color(colors.fg)
                .child(line[current_idx..line_len].to_string()),
        );
    }

    div()
        .flex()
        .flex_row()
        .font_family("monospace")
        .text_xs()
        .when(is_word_wrap, |s| s.flex_wrap())
        .children(spans)
        .into_any_element()
}
