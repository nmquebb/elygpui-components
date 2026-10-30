use std::{cell::Cell, rc::Rc};

use gpui::{
    Context, FocusHandle, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement as _,
    Render, Styled as _, TestAppContext, Window, div, px,
};

use super::CodeBlock;
use crate::{primitives::FocusScope, theme::Theme};

struct Preview {
    code: String,
    opened: Rc<Cell<usize>>,
    focus: FocusHandle,
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let opened = self.opened.clone();
        FocusScope::new(&self.focus).root().child(
            div().w(px(768.)).child(
                CodeBlock::new("code", self.code.clone())
                    .language("rust")
                    .preview(move |_, _| opened.set(opened.get() + 1)),
            ),
        )
    }
}

#[gpui::test]
fn previews_stay_bounded_and_open_by_pointer_and_keyboard(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_core::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let opened = Rc::new(Cell::new(0));
    let observed = opened.clone();
    let (_, visual) = cx.add_window_view(move |window, cx| {
        window.activate_window();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Preview {
            code: "let value = 42;\n".repeat(100_000),
            opened: observed,
            focus,
        }
    });
    visual.run_until_parked();
    let expand = visual
        .debug_bounds("code-preview-expand")
        .expect("a preview must offer Open in tab");
    assert!(expand.bottom() < px(430.), "hidden lines must not lay out");
    visual.simulate_click(expand.center(), Modifiers::none());
    visual.run_until_parked();
    assert_eq!(opened.get(), 1, "pointer opens the full-code viewer");
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_keystrokes("space");
    visual.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").unwrap(),
    });
    visual.run_until_parked();
    assert_eq!(
        visual.read_from_clipboard().and_then(|item| item.text()),
        Some("let value = 42;\n".repeat(100_000)),
        "Copy keeps the original source beyond the preview and trailing newline"
    );
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_keystrokes("space");
    visual.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").unwrap(),
    });
    visual.run_until_parked();
    assert_eq!(opened.get(), 2, "keyboard opens the same viewer");
}
