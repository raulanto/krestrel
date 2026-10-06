use ely_gpui_component::buttons::{Button, ButtonVariant, IconButton};
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{
    ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled, Window, div, px,
};

use super::state::DiagramViewMode;
use crate::theme::{ThemeExt as _, h_flex};

pub struct DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F9: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F10: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub on_select_types: F1,
    pub on_select_data: F2,
    pub on_zoom_in: F3,
    pub on_zoom_out: F4,
    pub on_reset_zoom: F5,
    pub on_fit_view: F6,
    pub on_expand_all: F7,
    pub on_collapse_all: F8,
    pub on_export_svg: F9,
    pub on_export_png: F10,
}

#[derive(IntoElement)]
pub struct DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F9: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F10: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    zoom: f32,
    view_mode: DiagramViewMode,
    actions: DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>,
}

impl<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>
    DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F9: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F10: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub fn new(
        zoom: f32,
        view_mode: DiagramViewMode,
        actions: DiagramToolbarActions<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>,
    ) -> Self {
        Self {
            zoom,
            view_mode,
            actions,
        }
    }
}

impl<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10> RenderOnce
    for DiagramToolbar<F1, F2, F3, F4, F5, F6, F7, F8, F9, F10>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F4: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F5: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F6: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F7: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F8: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F9: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F10: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let zoom_pct = (self.zoom * 100.0).round() as i32;
        let view_mode = self.view_mode;
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
            // View Mode Selector (Tipos / Datos)
            .child(
                h_flex()
                    .items_center()
                    .p_0p5()
                    .rounded_md()
                    .bg(colors.bg.opacity(0.7))
                    .border_1()
                    .border_color(colors.border)
                    .gap_0p5()
                    .child(
                        Button::new("diag-mode-types", "Tipos")
                            .variant(if view_mode == DiagramViewMode::Types {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .size(ControlSize::Sm)
                            .on_click(actions.on_select_types),
                    )
                    .child(
                        Button::new("diag-mode-data", "Datos")
                            .variant(if view_mode == DiagramViewMode::Data {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .size(ControlSize::Sm)
                            .on_click(actions.on_select_data),
                    ),
            )
            // Separator
            .child(div().w(px(1.)).h(px(18.)).mx_1().bg(colors.border))
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
