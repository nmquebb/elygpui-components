# Changelog

What changes for an app built on Ely, newest first. An entry names the crates it touches. Versions follow semver: below 1.0, a breaking change raises the minor version, and CI's `semver` job fails one that does not.

## Unreleased

- `ely-documents`: `MarkdownListContext::wrap` and `MarkdownRenderer::list_depth` let virtual fragments retain list marker sizing, task decoration and nested bullet styles; `MarkdownRenderer::quote` shares the quote frame with virtual fragments.

- `ely-agent`: `ToolCallGroup::expanded` lets a host own expansion while retaining the group's header, keyboard interaction and accessibility state.

- `ely-editor`, `ely-documents`: large highlighted text normalizes only active style spans at each boundary, preserving overlap order while avoiding quadratic scans across unrelated spans.

- `ely-chat`: `MessageList::list_state` accepts externally spliced rows, allowing block-level virtualization while preserving message arrival counts and follow-newest controls.

- `ely-documents`: dark Markdown code blocks use a faint lighter panel, including while editing; light panels retain their theme fill.

- `ely-documents`: task checkboxes align with their first text line in visual, preview and split Markdown, including wrapped and nested tasks.

- `ely-documents`: visual Markdown tasks toggle through document undo, and rendered code blocks have a top-right Copy with in-place feedback; blank space below the document continues writing.

The first release on crates.io.

- 31 library crates: `ely-core` (chapters 1–14, theme, motion, i18n), one crate per vertical chapter from `ely-charts` to `ely-rendering`, `ely-desktop` (tray, Dock badge, Omarchy), and `ely-gpui-component`, which re-exports them all under their chapter names.
- gpui, gpui_platform and gpui_web from `gpui-pre` `=0.3.9`, the snapshot GPUI Kit 0.8.0 pins, so Ely and Kit share a window.
- Every public item documented, each component with an example that compiles and runs; `api/` holds each crate's public API.
- Every focus stop has a role and a name for screen readers; `gallery --audit` walks them on Windows.
- Messages in English, Chinese, German, French, Japanese and Hebrew; the last four machine translated, awaiting native review.
- Icons link by use and fonts by feature (`fonts`, on by default); `languages` adds `Combobox::languages`.
