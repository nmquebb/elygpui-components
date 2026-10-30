use gpui::{
    App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _,
    div, linear_color_stop, linear_gradient,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    theme::{ActiveTheme as _, ControlSize},
};

use super::FOLD_AFTER;

const PREVIEW_BYTES: usize = 4096;

pub(super) fn end(code: &str) -> usize {
    let bytes = code.floor_char_boundary(code.len().min(PREVIEW_BYTES));
    code[..bytes]
        .match_indices('\n')
        .nth(FOLD_AFTER - 1)
        .map_or(bytes, |(at, _)| at)
}

pub(super) fn fade(surface: Hsla) -> impl IntoElement {
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .w_full()
        .h_6()
        .bg(linear_gradient(
            180.,
            linear_color_stop(surface.opacity(0.), 0.),
            linear_color_stop(surface, 1.),
        ))
}

pub(super) fn expand(id: ElementId, open: Run, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px_1()
        .py_0p5()
        .border_t_1()
        .border_color(cx.theme().colors.border)
        .child(
            div().debug_selector(|| "code-preview-expand".into()).child(
                Button::new((id, "expand"), "Open in tab")
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Md)
                    .on_click(move |_, window, cx| open(window, cx)),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::end;

    #[test]
    fn small_previews_keep_all_text() {
        for source in ["", "fn main() {}", "one\ntwo\n"] {
            assert_eq!(&source[..end(source)], source);
        }
    }

    #[test]
    fn multiline_previews_stop_after_fourteen_lines() {
        let source = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n";
        assert_eq!(
            &source[..end(source)],
            "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13"
        );
    }

    #[test]
    fn single_lines_and_blank_tails_are_bounded() {
        for source in ["word ".repeat(250_000), "\n".repeat(1_000_000)] {
            assert!(end(&source) <= 4096, "preview work must stay bounded");
            assert!(
                end(&source) < source.len(),
                "full source still needs opening"
            );
        }
    }

    #[test]
    fn byte_limit_keeps_multibyte_characters_whole() {
        let source = "a".repeat(4095) + "🌲 tail";
        assert_eq!(end(&source), 4095);
        assert!(source.is_char_boundary(end(&source)));
    }
}
