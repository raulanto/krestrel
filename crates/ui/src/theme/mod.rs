//! Theme adapter for Kestrel.
//!
//! Centralizes theme management using `ely-gpui-component` as the single source of truth.
//! Provides color tokens, semantic helpers (HTTP methods, status codes, syntax), and layout primitives.

pub use ely_gpui_component::theme::{
    ActiveTheme, ControlSize, IconSize, Mode, Palette, Radius, SyntaxTheme, TextSize, Theme,
};
use gpui::{App, Context, Div, Hsla, Styled, Window, div, rgb};
use kestrel_core::HttpMethod;

/// Extension trait providing uniform `.theme()` and `.colors()` access across GPUI contexts.
pub trait ThemeExt {
    fn theme(&self) -> &Theme;
    fn colors(&self) -> &Palette {
        &self.theme().colors
    }
}

impl<T> ThemeExt for Context<'_, T> {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl ThemeExt for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl ThemeExt for Window {
    fn theme(&self) -> &Theme {
        unimplemented!("Access theme through cx or app")
    }
}

/// Horizontal flex container helper (`div().flex().flex_row()`).
pub fn h_flex() -> Div {
    div().flex().flex_row()
}

/// Vertical flex container helper (`div().flex().flex_col()`).
pub fn v_flex() -> Div {
    div().flex().flex_col()
}

/// Helper for HTTP method badge colors (background, foreground, label).
pub fn http_method_badge(method: HttpMethod, _colors: &Palette) -> (Hsla, Hsla, &'static str) {
    let (bg_rgb, label) = match method {
        HttpMethod::GET => (0x22c55e, "GET"),
        HttpMethod::POST => (0xeab308, "POST"),
        HttpMethod::PUT => (0x3b82f6, "PUT"),
        HttpMethod::DELETE => (0xef4444, "DEL"),
        HttpMethod::PATCH => (0xa855f7, "PATCH"),
        HttpMethod::HEAD => (0x6b7280, "HEAD"),
        HttpMethod::OPTIONS => (0x6b7280, "OPT"),
    };

    let bg: Hsla = rgb(bg_rgb).into();
    let fg: Hsla = if method == HttpMethod::POST {
        rgb(0x000000).into()
    } else {
        rgb(0xffffff).into()
    };

    (bg, fg, label)
}

/// Helper for HTTP status code colors.
pub fn status_color(status: u16, colors: &Palette) -> Hsla {
    match status {
        200..=299 => colors.success,
        300..=399 => colors.info,
        400..=499 => colors.warning,
        500..=599 => colors.danger,
        _ => colors.fg_muted,
    }
}

/// Helper for JSON/syntax data type colors.
pub fn syntax_type_color(type_kind: &str, colors: &Palette) -> Hsla {
    match type_kind {
        "string" => colors.syntax.string,
        "number" => colors.syntax.number,
        "boolean" => colors.syntax.keyword,
        "null" => colors.syntax.constant,
        "key" | "property" => colors.syntax.property,
        _ => colors.fg,
    }
}
