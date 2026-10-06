use ely_gpui_component::buttons::{Button, ButtonVariant, IconButton};
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{
    ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled, Window, div, px,
};

use crate::theme::{ThemeExt as _, h_flex};

pub struct DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub on_zoom_in: F1,
    pub on_zoom_out: F2,
    pub on_reset_zoom: F3,
    pub on_fit_view: F4,
    pub on_expand_all: F5,
    pub on_collapse_all: F6,
    pub on_export_svg: F7,
    pub on_export_png: F8,
}

#[derive(IntoElement)]
pub struct DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    zoom: f32,
    actions: DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8>,
}

impl<F1, F2, F3, F4, F5, F6, F7, F8> DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub fn new(zoom: f32, actions: DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8>) -> Self {
        Self { zoom, actions }
    }
}

impl<F1, F2, F3, F4, F5, F6, F7, F8> RenderOnce for DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let zoom_pct = (self.zoom * 100.0).round() as i32;
        let actions = self.actions;

        h_flex()
            .h(px(36.))
            .px_2()
            .items_center()
            .gap_1p5()
            .bg(colors.surface.opacity(0.95))
            .border_1()
            .border_color(colors.border)
            .rounded_md()
            .shadow_md()
            // Zoom controls
            .child(
                IconButton::new("diag-zoom-out", IconName::Minus)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_zoom_out),
            )
            .child(
                div()
                    .w(px(46.))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .text_center()
                    .child(format!("{}%", zoom_pct)),
            )
            .child(
                IconButton::new("diag-zoom-in", IconName::Plus)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_zoom_in),
            )
            .child(
                IconButton::new("diag-reset-zoom", IconName::RotateCcw)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_reset_zoom),
            )
            .child(
                IconButton::new("diag-fit-view", IconName::Maximize2)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_fit_view),
            )
            // Separator
            .child(div().w(px(1.)).h(px(18.)).mx_1().bg(colors.border))
            // Expand / Collapse controls
            .child(
                IconButton::new("diag-expand-all", IconName::FolderOpen)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_expand_all),
            )
            .child(
                IconButton::new("diag-collapse-all", IconName::Folder)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_collapse_all),
            )
            // Separator
            .child(div().w(px(1.)).h(px(18.)).mx_1().bg(colors.border))
            // Export controls
            .child(
                Button::new("diag-btn-svg", "Exportar SVG")
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_export_svg),
            )
            .child(
                Button::new("diag-btn-png", "Exportar PNG")
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(actions.on_export_png),
            )
    }
}
