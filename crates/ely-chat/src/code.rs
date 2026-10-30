use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, HighlightStyle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, StyledText, Window, div, prelude::*,
    relative,
};

use crate::{
    buttons::{Button, ButtonVariant, CopyButton, IconButton},
    editor::code_colors,
    forms::Run,
    layout::on_axis,
    primitives::{Disclosure, IconName},
    theme::{ActiveTheme, ControlSize, Radius, Syntax, TextSize},
};

/// Lines a block shows before it folds.
const FOLD_AFTER: usize = 14;

/// Where `code` folds after `lines`: the byte end of the lines kept, when there are more.
pub(crate) fn fold_at(code: &str, lines: usize) -> Option<usize> {
    code.match_indices('\n')
        .nth(lines.checked_sub(1)?)
        .map(|(at, _)| at)
        .filter(|at| code[at + 1..].contains(|c: char| !c.is_whitespace()))
}

/// Code in a message: its language, a copy button, and the owner's download, run and apply; long code folds to a few lines until opened.
///
/// ```
/// use ely_chat::CodeBlock;
///
/// let code = CodeBlock::new("code", "fn main() {}").language("rust").on_apply(|_, _| {});
/// ```
#[derive(IntoElement)]
pub struct CodeBlock {
    id: ElementId,
    code: SharedString,
    language: Option<SharedString>,
    on_download: Option<Run>,
    on_run: Option<Run>,
    on_apply: Option<Run>,
}

impl CodeBlock {
    /// A block under `id` of `code`.
    pub fn new(id: impl Into<ElementId>, code: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            code: code.into(),
            language: None,
            on_download: None,
            on_run: None,
            on_apply: None,
        }
    }

    /// The language named in its header, such as `rust`; `code` unless set.
    pub fn language(mut self, language: impl Into<SharedString>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Offers to save the code as a file.
    pub fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }

    /// Offers to run the code.
    pub fn on_run(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_run = Some(Rc::new(handler));
        self
    }

    /// Puts the code where it belongs, such as into the open file.
    pub fn on_apply(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_apply = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let opened = *open.read(cx);
        let code = self.code.trim_end_matches('\n').to_string();
        let fold = fold_at(&code, FOLD_AFTER).filter(|_| !opened);
        let hidden = fold.map(|at| code[at..].lines().count() - 1);
        let shown = fold.map_or(code.as_str(), |at| &code[..at]).to_string();
        let shown: SharedString = shown.into();
        let highlighted = window.use_keyed_state((self.id.clone(), "highlights"), cx, |_, cx| {
            CodeColors::new(shown.clone(), cx)
        });
        highlighted.update(cx, |colors, cx| colors.refresh(shown.clone(), cx));
        let styles = highlighted.read(cx).styles.clone();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let action = |key: &str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((self.id.clone(), key.to_string()), icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .on_click(move |_, window, cx| run(window, cx))
            })
        };
        let lines = code.lines().count();
        let toggle = open.clone();
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.sunken)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .pl_3()
                    .pr_1()
                    .py_0p5()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(self.language.clone().unwrap_or_else(|| "code".into())),
                    )
                    .children(action("run", IconName::Play, "Run", self.on_run))
                    .children(action("apply", IconName::Check, "Apply", self.on_apply))
                    .children(action(
                        "download",
                        IconName::Download,
                        "Download",
                        self.on_download,
                    ))
                    .child(CopyButton::new((self.id.clone(), "copy"), code.clone())),
            )
            .child(
                on_axis(div().id((self.id.clone(), "code")))
                    .overflow_x_scroll()
                    .px_3()
                    .py_2()
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Sm))
                    .line_height(relative(1.55))
                    .text_color(colors.syntax.variable)
                    .whitespace_nowrap()
                    .child(StyledText::new(shown).with_highlights(styles.iter().cloned())),
            )
            .when(lines > FOLD_AFTER, |block| {
                block.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1()
                        .py_0p5()
                        .border_t_1()
                        .border_color(colors.border)
                        .child(
                            Button::new(
                                (self.id.clone(), "fold"),
                                match hidden {
                                    Some(hidden) => format!("Show {hidden} more lines"),
                                    None => "Show less".to_string(),
                                },
                            )
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |open, cx| {
                                    *open = !*open;
                                    log::info!("code block: open {open}");
                                    cx.notify();
                                })
                            }),
                        )
                        .child(Disclosure::new((self.id.clone(), "chevron"), opened)),
                )
            })
    }
}

struct CodeColors {
    source: SharedString,
    syntax: Syntax,
    styles: Rc<[(Range<usize>, HighlightStyle)]>,
}

impl CodeColors {
    fn new(source: SharedString, cx: &App) -> Self {
        Self {
            styles: highlights(&source, cx).into(),
            source,
            syntax: cx.theme().colors.syntax.clone(),
        }
    }

    fn refresh(&mut self, source: SharedString, cx: &App) {
        if self.source != source || self.syntax != cx.theme().colors.syntax {
            *self = Self::new(source, cx);
        }
    }
}

fn highlights(code: &str, cx: &App) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut styles = code_colors(code, cx);
    let base = cx.theme().colors.syntax.variable;
    // GPUI blends highlight colors; only an opaque inherited color is redundant.
    if base.a == 1.0 {
        let inherited = HighlightStyle {
            color: Some(base),
            ..Default::default()
        };
        styles.retain(|(_, style)| *style != inherited);
    }
    styles
}

#[cfg(test)]
mod tests {
    use super::{CodeColors, fold_at, highlights};
    use crate::theme::{ActiveTheme, Mode, Theme};
    use gpui::{TestAppContext, TextStyle};

    #[gpui::test]
    fn syntax_colors_preserve_inherited_and_translucent_palettes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            for mode in [Mode::Light, Mode::Dark] {
                Theme::set_mode_now(mode, cx);
                for alpha in [1.0, 0.45] {
                    cx.global_mut::<Theme>().colors.syntax.variable.a = alpha;
                    let syntax = cx.theme().colors.syntax.clone();
                    let base = TextStyle {
                        color: syntax.variable,
                        ..Default::default()
                    };
                    let source = "let word = 42; // comment";
                    let styles = highlights(source, cx);
                    let expected = [
                        (0..3, Some(syntax.keyword)),
                        (3..4, None),
                        (4..8, Some(syntax.variable)),
                        (8..9, None),
                        (9..10, Some(syntax.operator)),
                        (10..11, None),
                        (11..13, Some(syntax.number)),
                        (13..14, Some(syntax.punctuation)),
                        (14..15, None),
                        (15..source.len(), Some(syntax.comment)),
                    ];
                    for (range, color) in expected {
                        let expected = color.map_or(base.color, |color| base.color.blend(color));
                        for byte in range {
                            let actual = styles
                                .iter()
                                .find(|(span, _)| span.contains(&byte))
                                .map_or(base.color, |(_, style)| {
                                    base.clone().highlight(*style).color
                                });
                            assert_eq!(actual, expected, "{mode:?} alpha={alpha} byte={byte}");
                        }
                    }
                }
            }
        });
    }

    #[gpui::test]
    fn code_colors_follow_replacements_and_palette_changes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            let mut colors = CodeColors::new("let value".into(), cx);
            for source in ["let value", "123 value", "123 value", "let value"] {
                for mode in [Mode::Dark, Mode::Light] {
                    Theme::set_mode_now(mode, cx);
                    colors.refresh(source.into(), cx);
                    let syntax = &cx.theme().colors.syntax;
                    let expected = if source.starts_with("let") {
                        syntax.keyword
                    } else {
                        syntax.number
                    };
                    assert_eq!(colors.source.as_ref(), source);
                    for byte in 0..source.len() {
                        let actual = colors
                            .styles
                            .iter()
                            .find(|(range, _)| range.contains(&byte))
                            .map_or(syntax.variable, |(_, style)| {
                                TextStyle {
                                    color: syntax.variable,
                                    ..Default::default()
                                }
                                .highlight(*style)
                                .color
                            });
                        assert_eq!(actual, if byte < 3 { expected } else { syntax.variable });
                    }
                }
            }
        });
    }

    #[test]
    fn long_code_folds_after_its_lines() {
        assert_eq!(fold_at("a\nb\nc", 2), Some(3));
        assert_eq!(fold_at("a\nb", 2), None, "two lines fit");
        assert_eq!(
            fold_at("a\nb\n\n", 2),
            None,
            "blank lines past the fold do not count"
        );
        assert_eq!(fold_at("a", 0), None);
    }
}
