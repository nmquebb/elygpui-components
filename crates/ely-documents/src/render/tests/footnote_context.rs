//! Fragment typography and geometry match a whole footnote through nested containers.

use crate::{
    MarkdownBlock, MarkdownContext, MarkdownListContext, MarkdownRenderer,
    theme::{ActiveTheme as _, Mode, TextSize, Theme},
};
use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, RenderOnce,
    Styled as _, TestAppContext, Window, div, px, rems,
};
use std::{cell::RefCell, rc::Rc};

type Styles = Rc<RefCell<Vec<(String, Pixels, Pixels)>>>;

#[derive(IntoElement)]
struct Probe(String, Styles);

impl RenderOnce for Probe {
    fn render(self, window: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        self.1.borrow_mut().push((
            self.0.clone(),
            window.text_style().font_size.to_pixels(window.rem_size()),
            window.line_height(),
        ));
        div()
            .debug_selector(move || self.0.clone())
            .w_full()
            .h(rems(1.5))
    }
}

struct Page {
    split: bool,
    styles: Styles,
}

fn leaf(name: &str) -> MarkdownBlock {
    MarkdownBlock::Code {
        language: None,
        text: name.into(),
        from: 0,
    }
}

fn renderer(
    id: &'static str,
    blocks: Vec<MarkdownBlock>,
    split: bool,
    styles: Styles,
) -> MarkdownRenderer {
    MarkdownRenderer::blocks(id, Rc::from(blocks))
        .text_size(TextSize::Md)
        .inherit_body_style(split)
        .code(move |_, _, text| Probe(text.to_string(), styles.clone()).into_any_element())
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.split {
            div()
                .flex()
                .flex_col()
                .children(
                    ["first", "nested", "last"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| {
                            let mut contexts = vec![(
                                MarkdownContext::Footnote {
                                    label: "123456789".into(),
                                    continuation: i > 0,
                                },
                                px(0.),
                            )];
                            if i == 1 {
                                contexts.push((
                                    MarkdownContext::List(MarkdownListContext {
                                        depth: 0,
                                        number: Some(123456789),
                                        checked: None,
                                        continuation: false,
                                    }),
                                    px(0.),
                                ));
                                contexts.push((MarkdownContext::Quote, px(0.)));
                            }
                            MarkdownRenderer::wrap_fragment(
                                renderer(name, vec![leaf(name)], true, self.styles.clone()),
                                contexts,
                                TextSize::Md,
                            )
                        }),
                )
                .into_any_element()
        } else {
            renderer(
                "whole",
                vec![MarkdownBlock::Footnote {
                    label: "123456789".into(),
                    body: vec![
                        leaf("first"),
                        MarkdownBlock::List {
                            start: Some(123456789),
                            items: vec![(None, vec![MarkdownBlock::Quote(vec![leaf("nested")])])],
                        },
                        leaf("last"),
                    ],
                }],
                false,
                self.styles.clone(),
            )
            .into_any_element()
        };
        div().flex().items_start().child(
            div()
                .debug_selector(|| "footnote-stack".into())
                .w(px(500.))
                .text_size(cx.theme().text_size(TextSize::Xs))
                .line_height(cx.theme().text_size(TextSize::Xs) * 1.6)
                .child(body),
        )
    }
}

#[gpui::test]
fn footnote_fragments_inherit_small_text_and_outer_line_height_through_lists(
    cx: &mut TestAppContext,
) {
    cx.update(Theme::init);
    for mode in [Mode::Light, Mode::Dark] {
        cx.update(|cx| Theme::set_mode_now(mode, cx));
        let styles = Styles::default();
        let (view, visual) = cx.add_window_view(|_, _| Page {
            split: false,
            styles: styles.clone(),
        });
        visual.run_until_parked();
        let selectors = ["first", "nested", "last", "footnote-stack"];
        let whole = selectors.map(|s| visual.debug_bounds(s).expect("whole footnote probe"));
        let expected = visual.update(|window, cx| {
            (
                cx.theme()
                    .text_size(TextSize::Sm)
                    .to_pixels(window.rem_size()),
                (cx.theme()
                    .text_size(TextSize::Md)
                    .to_pixels(window.rem_size())
                    * 1.6)
                    .round(),
            )
        });
        assert!(
            styles
                .borrow()
                .iter()
                .all(|(_, size, height)| (*size, *height) == expected),
            "whole styles {:?}, expected {:?}",
            styles.borrow(),
            expected
        );
        assert_eq!(
            whole[1].top(),
            whole[0].bottom(),
            "footnote bodies have no gap"
        );
        assert!(
            (whole[2].top() - whole[1].bottom()).abs() <= px(1.),
            "only fractional marker rounding separates the probes"
        );
        assert!(
            whole[0].left() > px(50.),
            "wide label reserves its real gutter"
        );
        assert!(
            whole[1].left() > whole[0].left() + px(70.),
            "nested ordinal and quote retain indentation"
        );
        styles.borrow_mut().clear();
        view.update(visual, |view, cx| {
            view.split = true;
            cx.notify();
        });
        visual.run_until_parked();
        assert_eq!(
            selectors.map(|s| visual.debug_bounds(s).expect("fragment probe")),
            whole,
            "{mode:?}"
        );
        assert!(
            styles
                .borrow()
                .iter()
                .all(|(_, size, height)| (*size, *height) == expected),
            "fragment body typography: {:?}",
            styles.borrow()
        );
        visual.update(|window, _| window.remove_window());
    }
}
