use gpui::{Modifiers, TestAppContext};

use super::{MarkdownMode, open, text};

#[gpui::test]
fn a_task_press_toggles_only_its_marker_and_undo_restores_it(cx: &mut TestAppContext) {
    let source = "# Tasks\n\n- [ ] parent\n  - [x] nested\n- [ ] last\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let task = cx
        .debug_bounds("markdown-task-1")
        .expect("nested interactive task");
    cx.simulate_click(task.center(), Modifiers::none());
    assert_eq!(
        text(&document, cx),
        "# Tasks\n\n- [ ] parent\n  - [ ] nested\n- [ ] last\n"
    );
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(&document, cx), source);
}

#[gpui::test]
fn code_copy_leaves_document_text_and_caret_unchanged(cx: &mut TestAppContext) {
    let source = "```sh\nprintf 'café'\necho done\n```\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let copy = cx.debug_bounds("markdown-code-copy").expect("snippet Copy");
    cx.simulate_click(copy.center(), Modifiers::none());
    assert_eq!(
        cx.read(|cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some("printf 'café'\necho done")
    );
    assert_eq!(text(&document, cx), source);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
}

#[gpui::test]
fn toggling_after_live_typing_keeps_the_draft_and_maps_the_task(cx: &mut TestAppContext) {
    let (document, cx) = open("Intro\n\n- [ ] ship\n", MarkdownMode::Visual, cx);
    super::press(0, Modifiers::none(), cx);
    cx.simulate_input(" café");
    let task = cx
        .debug_bounds("markdown-task-0")
        .expect("task after draft");
    cx.simulate_click(task.center(), Modifiers::none());
    assert_eq!(text(&document, cx), "Intro café\n\n- [x] ship\n");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(&document, cx), "Intro café\n\n- [ ] ship\n");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(&document, cx), "Intro\n\n- [ ] ship\n");
}

#[gpui::test]
fn an_unnamed_fence_keeps_copy_in_the_top_right(cx: &mut TestAppContext) {
    let (_, cx) = open("```\nsnippet\n```\n", MarkdownMode::Visual, cx);
    let code = cx.debug_bounds("markdown-code").expect("code block");
    let copy = cx.debug_bounds("markdown-code-copy").expect("Copy");
    assert!(copy.center().x > code.center().x);
    assert!(copy.center().y < code.center().y);
}

#[gpui::test]
fn preview_tasks_are_read_only_and_snippets_still_copy(cx: &mut TestAppContext) {
    let source = "- [ ] ship\n\n```sh\necho done\n```\n";
    let (document, cx) = open(source, MarkdownMode::Preview, cx);
    assert!(cx.debug_bounds("markdown-task-0").is_none());
    let copy = cx
        .debug_bounds("markdown-code-copy")
        .expect("read-only Copy");
    cx.simulate_click(copy.center(), Modifiers::none());
    assert_eq!(text(&document, cx), source);
    assert_eq!(
        cx.read(|cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some("echo done")
    );
}

#[gpui::test]
fn a_task_box_on_the_next_line_keeps_following_task_identity(cx: &mut TestAppContext) {
    let source = "-\n  [ ] below\n- [ ] next\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let task = cx
        .debug_bounds("markdown-task-0")
        .expect("task below marker");
    cx.simulate_click(task.center(), Modifiers::none());
    assert_eq!(text(&document, cx), "-\n  [x] below\n- [ ] next\n");
}

#[gpui::test]
fn a_task_callback_from_an_old_read_does_not_change_new_text(cx: &mut TestAppContext) {
    let (document, cx) = open("- [ ] old\n", MarkdownMode::Visual, cx);
    let callback = document.update(cx, |document, cx| document.on_task(0, cx));
    document.update(cx, |document, cx| document.set_text("- [ ] new\n", cx));
    cx.update(|window, cx| callback(0, window, cx));
    assert_eq!(text(&document, cx), "- [ ] new\n");
}

#[gpui::test]
fn blank_document_space_continues_writing_without_an_add_control(cx: &mut TestAppContext) {
    let (document, cx) = open("First.\n", MarkdownMode::Visual, cx);
    let view = cx.update(|window, _| window.viewport_size());
    cx.simulate_click(
        gpui::point(view.width / 2.0, view.height - gpui::px(20.0)),
        Modifiers::none(),
    );
    cx.simulate_input("Second.");
    assert_eq!(text(&document, cx), "First.\n\nSecond.");
}

#[gpui::test]
fn task_boxes_center_on_the_first_text_line_in_every_rendered_mode(cx: &mut TestAppContext) {
    use crate::{
        DocumentLook,
        theme::{ActiveTheme, TextSize},
    };
    let source = format!(
        "- [ ] short\n- [x] {}\n  - [ ] nested\n",
        "wrapped words ".repeat(35)
    );
    for mode in [
        MarkdownMode::Visual,
        MarkdownMode::Preview,
        MarkdownMode::Split,
    ] {
        let (document, cx) = open(&source, mode, cx);
        for scale in [1.0, 1.5] {
            document.update(cx, |document, cx| {
                document.set_look(
                    DocumentLook {
                        scale,
                        ..DocumentLook::default()
                    },
                    cx,
                )
            });
            super::frames(cx);
            let line = cx.update(|window, cx| {
                cx.theme()
                    .text_size(TextSize::Base)
                    .to_pixels(window.rem_size())
                    * scale
                    * 1.6
            });
            for (task, body) in [
                (0, "markdown-list-body-0-0"),
                (1, "markdown-list-body-0-1"),
                (2, "markdown-list-body-1-0"),
            ] {
                let selector = Box::leak(format!("markdown-task-marker-{task}").into_boxed_str());
                let mark = cx.debug_bounds(selector).expect("task marker");
                let body = cx.debug_bounds(body).expect("task text");
                if task == 1 {
                    assert!(
                        body.size.height > line * 2.0,
                        "wrapped text and nested item"
                    );
                }
                let offset = mark.center().y - (body.top() + line / 2.0);
                assert!(
                    offset.abs() < gpui::px(0.5),
                    "{mode:?}, scale {scale}, task {task}: marker {mark:?}, text {body:?}, offset {offset:?}"
                );
            }
        }
    }
}
