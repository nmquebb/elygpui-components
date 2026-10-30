//! Fragment geometry matches a complete alert, including its one title and edge padding.

use crate::theme::{ActiveTheme as _, Mode, TextSize, Theme};
use crate::{AlertKind, MarkdownAlertContext, MarkdownBlock, MarkdownRenderer};
use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _,
    TestAppContext, Window, div, px, rems,
};
use std::rc::Rc;

struct Fragment {
    split: bool,
    kind: AlertKind,
}

fn leaf(name: &str) -> MarkdownBlock {
    MarkdownBlock::Code {
        language: None,
        text: name.into(),
        from: 0,
    }
}

fn renderer(id: &'static str, blocks: Vec<MarkdownBlock>) -> impl IntoElement {
    MarkdownRenderer::blocks(id, Rc::from(blocks))
        .text_size(TextSize::Md)
        .code(|_, _, text| {
            div()
                .debug_selector(move || text.to_string())
                .w_full()
                .h(rems(1.5))
                .into_any_element()
        })
}

impl Render for Fragment {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let names = ["first", "middle", "last"];
        let body = if self.split {
            div()
                .flex()
                .flex_col()
                .children(names.iter().enumerate().map(|(index, name)| {
                    MarkdownAlertContext {
                        kind: self.kind,
                        continuation: index > 0,
                        last: index == 2,
                    }
                    .wrap(
                        div()
                            .pt(if index == 0 { px(0.) } else { px(8.) })
                            .child(renderer(name, vec![leaf(name)])),
                        TextSize::Md,
                    )
                    .into_any_element()
                }))
                .into_any_element()
        } else {
            renderer(
                "whole",
                vec![MarkdownBlock::Alert {
                    kind: self.kind,
                    body: names.iter().map(|name| leaf(name)).collect(),
                }],
            )
            .into_any_element()
        };
        div().flex().items_start().child(
            div()
                .debug_selector(|| "alert-stack".into())
                .w(px(400.))
                .text_size(cx.theme().text_size(TextSize::Xs))
                .line_height(cx.theme().text_size(TextSize::Xs) * 1.6)
                .child(body),
        )
    }
}

#[gpui::test]
fn alert_fragments_keep_title_indent_inner_gaps_and_outer_padding(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for mode in [Mode::Light, Mode::Dark] {
        cx.update(|cx| Theme::set_mode_now(mode, cx));
        for kind in [
            AlertKind::Note,
            AlertKind::Tip,
            AlertKind::Important,
            AlertKind::Warning,
            AlertKind::Caution,
        ] {
            let (view, visual) = cx.add_window_view(move |_, _| Fragment { split: false, kind });
            visual.run_until_parked();
            let selectors = ["first", "middle", "last", "alert-stack"];
            let whole = selectors.map(|name| visual.debug_bounds(name).expect("whole alert probe"));
            assert_eq!(whole[0].left(), px(18.), "rule plus body indentation");
            assert_eq!(whole[1].top() - whole[0].bottom(), px(8.), "inner gap");
            assert_eq!(
                whole[3].bottom() - whole[2].bottom(),
                px(4.),
                "bottom padding only at end"
            );
            assert!(
                whole[0].top() >= px(28.),
                "one title precedes the first body"
            );
            view.update(visual, |view, cx| {
                view.split = true;
                cx.notify();
            });
            visual.run_until_parked();
            assert_eq!(
                selectors.map(|name| visual.debug_bounds(name).expect("fragment probe")),
                whole,
                "{kind:?} {mode:?}"
            );
            visual.update(|window, _| window.remove_window());
        }
    }
}
