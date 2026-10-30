//! Read-only display prefixes retain formatting and links without stale editing maps.

use super::Inline;

impl Inline {
    /// A UTF-8-safe display prefix of at most `max_bytes`, with clipped styles and links.
    /// Source mappings are omitted: a partial formatted span cannot independently map to its source.
    /// Keep the original inline for editing and source hit-testing. No ellipsis is added.
    ///
    /// ```
    /// use ely_documents::Inline;
    /// let line = Inline { text: "Aé🙂Z".into(), ..Default::default() };
    /// assert_eq!(line.display_prefix(4).text, "Aé");
    /// ```
    pub fn display_prefix(&self, max_bytes: usize) -> Self {
        let end = self.text.floor_char_boundary(max_bytes);
        Self {
            text: self.text[..end].to_owned(),
            styles: self
                .styles
                .iter()
                .filter(|(range, _)| range.start < end && range.end > range.start)
                .map(|(range, style)| (range.start..range.end.min(end), *style))
                .collect(),
            links: self
                .links
                .iter()
                .filter(|(range, _)| range.start < end && range.end > range.start)
                .map(|(range, url)| (range.start..range.end.min(end), url.clone()))
                .collect(),
            sources: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Inline;
    use crate::tree::{Node, Style, parse};

    #[test]
    fn prefixes_clip_unicode_styles_and_links_without_changing_the_original() {
        let line = Inline {
            text: "Aé🙂Z".into(),
            styles: vec![(0..8, Style::Strong), (1..7, Style::Emphasis)],
            links: vec![(1..8, "https://example.com".into())],
            sources: vec![],
        };
        let prefix = line.display_prefix(4);
        assert_eq!(prefix.text, "Aé");
        assert_eq!(
            prefix.styles,
            [(0..3, Style::Strong), (1..3, Style::Emphasis)]
        );
        assert_eq!(prefix.links, [(1..3, "https://example.com".into())]);
        assert_eq!(line.text, "Aé🙂Z");
        assert_eq!(line.styles[0].0, 0..8);
        assert_eq!(line.display_prefix(0), Inline::default());
        assert_eq!(line.display_prefix(usize::MAX), line);
    }

    #[test]
    fn display_prefixes_omit_editing_maps_and_leave_complete_source_unchanged() {
        let source = "**Alpha** [Béta](https://example.com) &amp; 😀";
        let blocks = parse(source);
        let Node::Paragraph(line) = &blocks[0] else {
            panic!("paragraph")
        };
        assert_eq!(line.text, "Alpha Béta & 😀");
        assert_eq!(line.source_at(source, 12), source.find("&amp;"));
        let shown = line.display_prefix(13);
        assert_eq!(shown.text, "Alpha Béta &");
        assert!(
            shown.sources.is_empty(),
            "a display prefix must not carry a stale editing map"
        );
        assert_eq!(shown.source_at(source, 12), None);
        assert_eq!(line.source_at(source, 12), source.find("&amp;"));
        for limit in [0, 1, 4, 8, 9, 13, 16, 18] {
            let shown = line.display_prefix(limit);
            assert!(shown.text.len() <= limit);
            assert!(shown.sources.is_empty());
        }
    }
}
