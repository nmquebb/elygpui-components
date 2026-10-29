use gpui::{ListAlignment, ListState};

use super::*;

struct Rows {
    messages: usize,
    keys: Vec<usize>,
    list: ListState,
    seen: Seen,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        let keys = self.keys.clone();
        seen.borrow_mut().clear();
        div().size_full().child(
            MessageList::new("rows", self.messages, move |ix, _, _| {
                seen.borrow_mut().push(keys[ix]);
                div().h(px(80.0)).w_full().into_any_element()
            })
            .list_state(self.list.clone()),
        )
    }
}

#[gpui::test]
fn external_rows_follow_newest_and_hold_the_reader_across_earlier_splices(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let seen = Seen::default();
    let noted = seen.clone();
    let (rows, cx) = cx.add_window_view(move |_, _| Rows {
        messages: 40,
        keys: (0..400).collect(),
        list: ListState::new(400, ListAlignment::Bottom, px(200.0)),
        seen: noted,
    });
    settle(cx);
    assert_eq!(newest(&seen), 399, "all rows belong to the message list");
    rows.update(cx, |rows, cx| {
        rows.keys.extend(400..410);
        rows.list.splice(400..400, 10);
        rows.messages += 1;
        cx.notify();
    });
    settle(cx);
    assert_eq!(newest(&seen), 409, "new rows follow at the newest");
    let middle = cx.update(|window, _| {
        let size = window.viewport_size();
        point(size.width / 2.0, size.height / 2.0)
    });
    cx.simulate_event(ScrollWheelEvent {
        position: middle,
        delta: ScrollDelta::Pixels(point(px(0.0), px(1200.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    let away = newest(&seen);
    assert!(away < 409);
    rows.update(cx, |rows, cx| {
        rows.keys.splice(0..0, 1000..1020);
        rows.list.splice(0..0, 20);
        cx.notify();
    });
    settle(cx);
    assert_eq!(
        newest(&seen),
        away,
        "earlier rows preserve the visible message"
    );
    rows.update(cx, |rows, cx| {
        rows.keys.extend(410..420);
        rows.list.splice(430..430, 10);
        rows.messages += 1;
        cx.notify();
    });
    settle(cx);
    assert_eq!(newest(&seen), away, "new messages hold the reader's place");
    let button = cx.update(|window, _| {
        let size = window.viewport_size();
        point(size.width / 2.0, size.height - px(28.0))
    });
    cx.simulate_click(button, Modifiers::none());
    settle(cx);
    assert_eq!(newest(&seen), 419, "Latest reaches the last row");
}
