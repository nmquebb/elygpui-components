//! Fenced code keeps its highlighting and source hit-testing beside a snippet Copy.

use gpui::{
    AnyElement, App, Hsla, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, StyledText, div,
    prelude::FluentBuilder as _,
};

use super::Draw;
use crate::{
    buttons::CopyButton,
    diagram::Diagram,
    editor::{code_colors, code_colors_with},
    theme::{ActiveTheme as _, Radius, TextSize, Theme},
};

impl Draw<'_> {
    pub(super) fn code_block(
        &self,
        language: &Option<String>,
        text: &str,
        from: usize,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let mermaid = language.as_deref() == Some("mermaid");
        if mermaid && !self.hooks.copy_code {
            let diagram = Diagram::new(self.key("diagram"), text.to_string());
            return self.point_code(diagram.into_any_element(), None, text, from);
        }
        if let Some(draw) = self.code.as_ref().filter(|_| !mermaid) {
            return draw(
                self.key("code"),
                language.clone().map(Into::into),
                text.to_string().into(),
            );
        }
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(div().flex_1().children(language.clone().map(|name| {
                div()
                    .text_size(theme.text_size(TextSize::Xs) * self.scale)
                    .text_color(colors.fg_subtle)
                    .child(name)
            })))
            .when(self.hooks.copy_code, |header| {
                header.child(
                    div()
                        .id(self.key("code-copy-slot"))
                        .debug_selector(|| "markdown-code-copy".into())
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(CopyButton::new(self.key("code-copy"), text.to_string())),
                )
            });
        let block = div()
            .debug_selector(|| "markdown-code".into())
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .bg(code_background(theme))
            .when(language.is_some() || self.hooks.copy_code, |block| {
                block.child(header)
            });
        if mermaid {
            let diagram = Diagram::new(self.key("diagram"), text.to_string());
            return self.point_code(block.child(diagram).into_any_element(), None, text, from);
        }
        let styles = match language.as_deref() {
            Some(name) => crate::forms::syntax_fenced(name)
                .map_or_else(Vec::new, |syntax| code_colors_with(text, syntax, cx)),
            None => code_colors(text, cx),
        };
        let code = StyledText::new(text.to_string()).with_highlights(styles);
        let layout = code.layout().clone();
        let block = block
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm) * self.scale)
            .text_color(colors.syntax.variable)
            .child(
                div()
                    .debug_selector(|| "markdown-code-text".into())
                    .child(code),
            )
            .into_any_element();
        self.point_code(block, Some(layout), text, from)
    }
}

/// A faint wash distinguishes dark snippets from their document surface.
pub(crate) fn code_background(theme: &Theme) -> Hsla {
    if theme.is_dark() {
        theme.colors.fg.opacity(0.04)
    } else {
        theme.colors.sunken
    }
}
