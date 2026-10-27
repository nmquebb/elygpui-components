//! Task markers write through the document history without opening a text field.

use std::rc::Rc;

use gpui::{Context, Window};

use super::{MarkdownDocument, edit::moved, parse::Parsed};
use crate::{MarkdownMode, render::OnTask};

impl MarkdownDocument {
    pub(super) fn on_task(&self, ix: usize, cx: &mut Context<Self>) -> OnTask {
        let (document, parsed) = (cx.entity().downgrade(), self.parsed.clone());
        Rc::new(move |task, window, cx| {
            let Some(mark) = parsed
                .blocks
                .get(ix)
                .and_then(|block| block.tasks.get(task))
                .copied()
            else {
                return log::warn!("markdown task: marker {task} no longer exists");
            };
            if let Err(error) = document.update(cx, |document, cx| {
                document.toggle_task(&parsed, mark, window, cx);
            }) {
                log::debug!("markdown task: document closed: {error}");
            }
        })
    }

    fn toggle_task(
        &mut self,
        parsed: &Parsed,
        mark: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mode != MarkdownMode::Visual
            || self.parsed.text != parsed.text
            || !self.read_now(cx)
        {
            return log::warn!("markdown task: drawing no longer reads the document");
        }
        let Some(mark) = moved(mark, &self.commit(cx)) else {
            return log::warn!("markdown task: typing replaced the pressed marker");
        };
        let next = match self.editor.read(cx).text().as_bytes().get(mark) {
            Some(b' ') => "x",
            Some(b'x' | b'X') => " ",
            _ => return log::warn!("markdown task: no checkbox at byte {mark}"),
        };
        self.editor.update(cx, |editor, cx| {
            editor.edit([(mark..mark + 1, next.to_string())], cx)
        });
        window.focus(&self.focus, cx);
        log::info!("markdown task: toggled marker at {mark}");
    }
}
