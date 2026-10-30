use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
    Window, div,
};

use super::{press, settle};
use crate::{ToolCallGroup, theme::Theme};

struct Group {
    controlled: bool,
    open: bool,
    requests: Vec<bool>,
}

impl Render for Group {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let group = ToolCallGroup::new("group", "Thinking")
            .child(div().debug_selector(|| "group-body".into()).h_8().w_full());
        div().size_full().child(if self.controlled {
            group.expanded(self.open, move |open, _, cx| {
                owner
                    .update(cx, |view, cx| {
                        view.requests.push(open);
                        cx.notify();
                    })
                    .unwrap();
            })
        } else {
            group
        })
    }
}

#[gpui::test]
fn controlled_groups_wait_for_the_owner_and_accept_external_changes(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Group {
        controlled: true,
        open: false,
        requests: Vec::new(),
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    settle(cx);
    for _ in 0..2 {
        press("enter", cx);
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.requests.clone()),
        [true, true]
    );
    assert!(cx.debug_bounds("group-body").is_none());
    view.update(cx, |view, cx| {
        view.open = true;
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("group-body").is_some());
    press("space", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.requests.clone()),
        [true, true, false]
    );
    assert!(cx.debug_bounds("group-body").is_some());
    view.update(cx, |view, cx| {
        view.open = false;
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("group-body").is_none());
}

#[gpui::test]
fn uncontrolled_groups_keep_their_keyboard_toggle(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Group {
        controlled: false,
        open: false,
        requests: Vec::new(),
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    settle(cx);
    assert!(cx.debug_bounds("group-body").is_none());
    press("enter", cx);
    assert!(cx.debug_bounds("group-body").is_some());
    press("space", cx);
    assert!(cx.debug_bounds("group-body").is_none());
}
