use std::time::Instant;

use ely_gpui_component::buttons::{Button, ButtonVariant};
use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled, div, px};
use kestrel_http::{RequestError, format_duration};

use crate::theme::{ThemeExt as _, h_flex, v_flex};

#[derive(IntoElement)]
pub struct IdleState;

impl RenderOnce for IdleState {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_3()
            .p_8()
            .child(
                div()
                    .p_4()
                    .rounded_full()
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .child(Icon::new(IconName::Globe).color(colors.fg_muted)),
            )
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .text_sm()
                    .text_color(colors.fg)
                    .child("Sin respuesta aún"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.fg_muted)
                    .child("Envía una solicitud (Ctrl+Enter) para ver los datos de respuesta aquí"),
            )
    }
}

#[derive(IntoElement)]
pub struct LoadingState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    started: Instant,
    on_cancel: F,
}

impl<F> LoadingState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(started: Instant, on_cancel: F) -> Self {
        Self { started, on_cancel }
    }
}

impl<F> RenderOnce for LoadingState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let elapsed = format_duration(self.started.elapsed());

        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(
                div()
                    .p_4()
                    .rounded_full()
                    .bg(colors.accent.opacity(0.1))
                    .border_1()
                    .border_color(colors.accent.opacity(0.3))
                    .child(Icon::new(IconName::RotateCcw).color(colors.accent)),
            )
            .child(
                v_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_sm()
                            .text_color(colors.fg)
                            .child("Enviando solicitud..."),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(format!("Tiempo transcurrido: {}", elapsed)),
                    ),
            )
            .child(
                Button::new("cancel-request-btn", "Cancelar")
                    .variant(ButtonVariant::Ghost)
                    .on_click(self.on_cancel),
            )
    }
}

#[derive(IntoElement)]
pub struct FailedState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    error: RequestError,
    on_retry: F,
}

impl<F> FailedState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(error: RequestError, on_retry: F) -> Self {
        Self { error, on_retry }
    }
}

impl<F> RenderOnce for FailedState<F>
where
    F: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .p_6()
            .child(
                v_flex()
                    .w(px(520.))
                    .p_5()
                    .rounded_lg()
                    .border_1()
                    .border_color(colors.danger.opacity(0.4))
                    .bg(colors.danger.opacity(0.06))
                    .gap_3()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2p5()
                            .child(Icon::new(IconName::X).color(colors.danger))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_sm()
                                    .text_color(colors.danger)
                                    .child(self.error.message.clone()),
                            ),
                    )
                    .children(self.error.details.as_ref().map(|d| {
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .p_3()
                            .rounded_md()
                            .bg(colors.surface)
                            .border_1()
                            .border_color(colors.border)
                            .child(d.clone())
                    }))
                    .child(
                        h_flex().justify_end().child(
                            Button::new("retry-request-btn", "Reintentar")
                                .variant(ButtonVariant::Primary)
                                .on_click(self.on_retry),
                        ),
                    ),
            )
    }
}

#[derive(IntoElement)]
pub struct EmptyBodyState;

impl RenderOnce for EmptyBodyState {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_2()
            .p_8()
            .child(Icon::new(IconName::Check).color(colors.success))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .text_sm()
                    .text_color(colors.fg)
                    .child("Respuesta sin cuerpo (0 bytes)"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.fg_muted)
                    .child("El servidor devolvió un código exitoso sin contenido en el cuerpo"),
            )
    }
}
