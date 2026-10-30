//! Alert frames share title, border and padding across independently virtualized body fragments.

use super::Draw;
use crate::AlertKind;
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme as _, IconSize, TextSize},
    tree::Node,
};
use gpui::{
    AnyElement, App, FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled as _, Window,
    div, prelude::*,
};

/// One fragment of an alert, retaining its rule and showing the title only at the start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownAlertContext {
    /// The alert's title, icon and border tone.
    pub kind: AlertKind,
    /// Hide the title and top padding after the first fragment.
    pub continuation: bool,
    /// Keep bottom padding only on the final fragment.
    pub last: bool,
}

impl MarkdownAlertContext {
    /// Decorates a body fragment; inner spacing belongs inside the frame.
    ///
    /// ```
    /// use ely_documents::{AlertKind, MarkdownAlertContext};
    /// let frame = MarkdownAlertContext { kind: AlertKind::Note, continuation: false, last: true };
    /// let alert = frame.wrap("one fragment", ely_core::theme::TextSize::Md);
    /// ```
    pub fn wrap(self, content: impl IntoElement, size: TextSize) -> impl IntoElement {
        Fragment {
            context: self,
            size,
            body: content.into_any_element(),
        }
    }
}

#[derive(IntoElement)]
struct Fragment {
    context: MarkdownAlertContext,
    size: TextSize,
    body: AnyElement,
}

impl RenderOnce for Fragment {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        frame(self.context, [self.body], cx)
            .text_size(cx.theme().text_size(self.size))
            .line_height(cx.theme().text_size(self.size) * 1.6)
    }
}

impl Draw<'_> {
    pub(super) fn alert(&self, kind: AlertKind, body: &[Node], cx: &App) -> AnyElement {
        frame(
            MarkdownAlertContext {
                kind,
                continuation: false,
                last: true,
            },
            self.nodes(body, cx),
            cx,
        )
        .into_any_element()
    }
}

fn frame(
    context: MarkdownAlertContext,
    body: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> gpui::Div {
    let colors = &cx.theme().colors;
    let (tone, icon) = match context.kind {
        AlertKind::Note => (colors.info, IconName::Info),
        AlertKind::Tip => (colors.success, IconName::Lightbulb),
        // The palette's violet, read at 4.5:1.
        AlertKind::Important => (colors.syntax.keyword, IconName::MessageSquareWarning),
        AlertKind::Warning => (colors.warning, IconName::TriangleAlert),
        AlertKind::Caution => (colors.danger, IconName::OctagonAlert),
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .when(!context.continuation, |frame| frame.pt_1())
        .when(context.last, |frame| frame.pb_1())
        .pl_4()
        .border_l_2()
        .border_color(tone)
        .when(!context.continuation, |frame| {
            frame.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(tone)
                    .child(Icon::new(icon).size(IconSize::Sm).color(tone))
                    .child(context.kind.title()),
            )
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .text_color(colors.fg)
                .children(body),
        )
}
