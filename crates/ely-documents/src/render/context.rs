//! Inherited frames around independently laid-out Markdown leaves.

use super::{MarkdownAlertContext, MarkdownListContext, alert, blocks, list};
use crate::theme::{ActiveTheme as _, TextSize};
use gpui::{
    AnyElement, App, IntoElement, ParentElement as _, Pixels, RenderOnce, SharedString,
    Styled as _, Window, div, prelude::*,
};

/// An enclosing Markdown container, ordered from outermost to innermost by fragment owners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownContext {
    /// A list item with its original marker geometry.
    List(MarkdownListContext),
    /// A quote's continuous rule and indentation.
    Quote,
    /// An alert's title, rule and edge padding.
    Alert(MarkdownAlertContext),
    /// A footnote's smaller body text and label gutter.
    Footnote {
        /// Displayed label from the complete document's reference numbering.
        label: SharedString,
        /// Reserve the label's width without repeating it after the first fragment.
        continuation: bool,
    },
}

#[derive(IntoElement)]
pub(super) struct Fragment {
    pub content: AnyElement,
    pub contexts: Vec<(MarkdownContext, Pixels)>,
    pub size: TextSize,
}

impl RenderOnce for Fragment {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let body = self
            .contexts
            .into_iter()
            .rev()
            .fold(self.content, |body, (context, gap)| {
                let body = match context {
                    MarkdownContext::List(item) => list::inherited(item, body).into_any_element(),
                    MarkdownContext::Quote => blocks::quote_frame([body], cx).into_any_element(),
                    MarkdownContext::Alert(alert) => {
                        alert::frame(alert, [body], cx).into_any_element()
                    }
                    MarkdownContext::Footnote {
                        label,
                        continuation,
                    } => footnote_frame(&label, continuation, [body], cx).into_any_element(),
                };
                if gap == gpui::px(0.) {
                    body
                } else {
                    div().pt(gap).child(body).into_any_element()
                }
            });
        div()
            .text_size(cx.theme().text_size(self.size))
            .line_height(cx.theme().text_size(self.size) * 1.6)
            .child(body)
    }
}

pub(super) fn footnote_frame(
    label: &str,
    continuation: bool,
    body: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> gpui::Div {
    div()
        .flex()
        .gap_2()
        .text_size(cx.theme().text_size(TextSize::Sm))
        .text_color(cx.theme().colors.fg_muted)
        .child(
            div()
                .flex_none()
                .when(continuation, |label| label.invisible())
                .child(format!("[{label}]")),
        )
        .child(div().flex_1().flex().flex_col().children(body))
}
