use gpui::{
    AnyElement, App, IntoElement, MouseButton, ParentElement, Role, Styled, StyledText, div,
    prelude::*,
};

use super::{Draw, point::point_lines};
use crate::{
    primitives::FocusRing as _,
    theme::{ActiveTheme, Radius, TextSize},
    tree::DocumentHeading,
};

/// The quote frame also decorates independently virtualized body fragments.
pub(super) fn quote_frame(body: impl IntoIterator<Item = AnyElement>, cx: &App) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pl_4()
        .border_l_2()
        .border_color(cx.theme().colors.border_strong)
        .text_color(cx.theme().colors.fg_muted)
        .children(body)
}

#[derive(IntoElement)]
pub(super) struct QuoteFrame(pub(super) AnyElement);

impl gpui::RenderOnce for QuoteFrame {
    fn render(self, _: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        quote_frame([self.0], cx)
    }
}

impl Draw<'_> {
    /// `[toc]`: each heading by depth; a press reveals it.
    pub(super) fn toc(&self, headings: &[DocumentHeading], cx: &App) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let top = headings.iter().map(|heading| heading.depth).min();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(headings.iter().enumerate().map(|(ix, heading)| {
                let entry = div()
                    .id(self.key("toc"))
                    .debug_selector(move || format!("markdown-toc-{ix}"))
                    .child(heading.title.clone());
                let entry = match heading.depth - top.expect("a heading is listed") {
                    0 => entry,
                    1 => entry.pl_4(),
                    2 => entry.pl_8(),
                    3 => entry.pl_12(),
                    4 => entry.pl_16(),
                    _ => entry.pl_20(),
                };
                let Some(reveal) = self.hooks.on_heading.clone() else {
                    return entry.text_color(colors.fg_muted).into_any_element();
                };
                let (at, pressed) = (heading.at, reveal.clone());
                entry
                    .role(Role::Link)
                    .aria_label(heading.title.clone())
                    .text_color(colors.link)
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    // On press: the layout may move before release.
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if event.modifiers.secondary() {
                            window.prevent_default();
                            cx.stop_propagation();
                            pressed(at, window, cx);
                        }
                    })
                    .on_click(move |event, window, cx| {
                        if !event.modifiers().secondary() {
                            reveal(at, window, cx);
                        }
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }

    /// Front matter as written, inside its box.
    pub(super) fn front_matter(&self, text: &str, from: usize, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let drawn = StyledText::new(text.to_string());
        let layout = drawn.layout().clone();
        let block = div()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .bg(theme.colors.sunken)
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm) * self.scale)
            .text_color(theme.colors.fg_muted)
            .child(drawn)
            .into_any_element();
        point_lines(
            block,
            layout,
            text,
            from,
            self.hooks.on_point,
            (false, false),
        )
    }
}
