//! List rows and continuation frames share marker sizing, indentation and task decoration.

use super::Draw;
use crate::{
    forms::Checkbox,
    i18n,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
    tree::Node,
};
use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

/// One enclosing list item around a fragment of its body. Depth is zero-based; number is the
/// item's displayed ordinal, or `None` for an unordered item. Continuations reserve the same
/// marker width without displaying it. Task decoration remains inherited by the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownListContext {
    /// Zero-based depth of this enclosing list.
    pub depth: usize,
    /// Displayed ordinal, or an unordered bullet when absent.
    pub number: Option<u64>,
    /// Task state; replaces the bullet or ordinal when present.
    pub checked: Option<bool>,
    /// Reserve the marker without drawing it after the first fragment.
    pub continuation: bool,
}

impl MarkdownListContext {
    /// Wrap any fragment at the Markdown body's text size, including custom code or artifacts.
    ///
    /// ```
    /// use ely_documents::MarkdownListContext;
    /// use ely_core::theme::TextSize;
    /// let item = MarkdownListContext { depth: 1, number: Some(3), checked: None, continuation: true };
    /// let row = item.wrap("continued body", TextSize::Md);
    /// ```
    pub fn wrap(self, content: impl IntoElement, size: TextSize) -> impl IntoElement {
        Contextual {
            context: self,
            size,
            content: content.into_any_element(),
        }
    }
}

#[derive(IntoElement)]
struct Contextual {
    context: MarkdownListContext,
    size: TextSize,
    content: AnyElement,
}

impl RenderOnce for Contextual {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let context = self.context;
        let marker = match context.checked {
            Some(done) => TaskMarker(div().flex().child(task_icon(done, cx)).into_any_element())
                .into_any_element(),
            None => marker(context.depth, context.number, cx),
        };
        row(
            div()
                .when(context.continuation, |slot| slot.invisible())
                .child(marker),
            self.content,
            context.checked == Some(true),
            cx,
        )
        .text_size(cx.theme().text_size(self.size))
        .line_height(cx.theme().text_size(self.size) * 1.6)
    }
}

fn row(marker: impl IntoElement, body: impl IntoElement, done: bool, cx: &App) -> Div {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .flex_none()
                .min_w_5()
                .flex()
                .justify_end()
                .child(marker),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .when(done, |body| body.text_color(cx.theme().colors.fg_subtle))
                .child(body),
        )
}

fn marker(depth: usize, number: Option<u64>, cx: &App) -> AnyElement {
    let marker = match number {
        Some(number) => div().child(format!("{number}.")),
        None => div()
            .debug_selector(move || format!("markdown-bullet-{depth}"))
            .child(match depth {
                0 => "•",
                1 => "◦",
                _ => "▪",
            }),
    };
    marker
        .text_color(cx.theme().colors.fg_muted)
        .into_any_element()
}

fn task_icon(done: bool, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    Icon::new(if done {
        IconName::SquareCheck
    } else {
        IconName::Square
    })
    .size(IconSize::Sm)
    .color(if done { colors.focus } else { colors.fg_subtle })
    .into_any_element()
}

impl Draw<'_> {
    /// A list; bullets turn with depth, as browsers draw them.
    pub(super) fn list(
        &self,
        start: &Option<u64>,
        items: &[(Option<bool>, Vec<Node>)],
        cx: &App,
    ) -> AnyElement {
        let depth = self.lists.get();
        self.lists.set(depth + 1);
        let list = div()
            .flex()
            .flex_col()
            .gap_1()
            .children(items.iter().enumerate().map(|(ix, (task, body))| {
                let marker: AnyElement = match (task, start) {
                    (Some(done), _) => self.task_marker(*done, body, cx),
                    (None, number) => marker(depth, number.map(|first| first + ix as u64), cx),
                };
                row(
                    marker,
                    div()
                        .debug_selector(move || format!("markdown-list-body-{depth}-{ix}"))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(self.nodes(body, cx)),
                    *task == Some(true),
                    cx,
                )
            }))
            .into_any_element();
        self.lists.set(depth);
        list
    }

    fn task_marker(&self, done: bool, body: &[Node], cx: &App) -> AnyElement {
        let task = self.tasks.get();
        self.tasks.set(task + 1);
        let marker = match self.hooks.on_task {
            Some(on_task) => {
                let on_task = on_task.clone();
                let name = body
                    .iter()
                    .find_map(|node| match node {
                        Node::Paragraph(inline) if !inline.text.is_empty() => {
                            Some(inline.text.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| i18n::text(cx, "block.todo", &[]).to_string());
                div()
                    .id(self.key("task-slot"))
                    .debug_selector(move || format!("markdown-task-{task}"))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        Checkbox::new(self.key("task"), done)
                            .name(name)
                            .on_change(move |_, window, cx| on_task(task, window, cx)),
                    )
                    .into_any_element()
            }
            None => task_icon(done, cx),
        };
        TaskMarker(
            div()
                .debug_selector(move || format!("markdown-task-marker-{task}"))
                .flex()
                .child(marker)
                .into_any_element(),
        )
        .into_any_element()
    }
}

#[derive(IntoElement)]
struct TaskMarker(AnyElement);

impl RenderOnce for TaskMarker {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .h(window.line_height())
            .flex()
            .items_center()
            .child(self.0)
    }
}
