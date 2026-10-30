//! Fragment frames retain the same marker geometry as complete nested lists.

use crate::{
    MarkdownBlock, MarkdownListContext, MarkdownRenderer,
    theme::{TextSize, Theme},
};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
    Window, div, px, rems,
};
use std::rc::Rc;

struct Fragment {
    split: bool,
    continuation: bool,
    quoted: bool,
}

fn contexts(continuation: bool) -> [MarkdownListContext; 2] {
    [
        MarkdownListContext {
            depth: 0,
            number: Some(123456789),
            checked: None,
            continuation,
        },
        MarkdownListContext {
            depth: 1,
            number: None,
            checked: Some(true),
            continuation,
        },
    ]
}

impl Render for Fragment {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let leaf = MarkdownBlock::Code {
            language: None,
            text: "probe".into(),
            from: 0,
        };
        let mut blocks: Rc<[MarkdownBlock]> = if self.split {
            vec![leaf].into()
        } else {
            vec![MarkdownBlock::List {
                start: Some(123456789),
                items: vec![(
                    None,
                    vec![MarkdownBlock::List {
                        start: None,
                        items: vec![(Some(true), vec![leaf])],
                    }],
                )],
            }]
            .into()
        };
        if self.quoted && !self.split {
            blocks = vec![MarkdownBlock::Quote(blocks.to_vec())].into();
        }
        let mut body = MarkdownRenderer::blocks("fragment", blocks)
            .text_size(TextSize::Md)
            .list_depth(if self.split { 2 } else { 0 })
            .code(|_, _, _| {
                div()
                    .debug_selector(|| "probe".into())
                    .w_full()
                    .h(rems(1.5))
                    .into_any_element()
            })
            .into_any_element();
        if self.split {
            for context in contexts(self.continuation).into_iter().rev() {
                body = context.wrap(body, TextSize::Md).into_any_element();
            }
        }
        if self.quoted && self.split {
            body = MarkdownRenderer::quote(body).into_any_element();
        }
        div().w(px(400.)).child(body)
    }
}

#[gpui::test]
fn continuation_frames_preserve_wide_number_and_task_indentation(cx: &mut TestAppContext) {
    check_geometry(cx, false);
}

#[gpui::test]
fn quote_fragments_keep_the_same_border_and_indent(cx: &mut TestAppContext) {
    check_geometry(cx, true);
}

fn check_geometry(cx: &mut TestAppContext, quoted: bool) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Fragment {
        split: false,
        continuation: false,
        quoted,
    });
    cx.run_until_parked();
    let whole = cx.debug_bounds("probe").unwrap();
    for continuation in [false, true] {
        view.update(cx, |view, cx| {
            view.split = true;
            view.continuation = continuation;
            cx.notify();
        });
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("probe").unwrap(),
            whole,
            "continuation={continuation}"
        );
    }
}
