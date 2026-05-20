mod analyzer;
mod escape;
mod list_compare;
mod markdown_preview;
mod text_compare;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(analyzer::TextAnalyzer::default()),
        Box::new(escape::EscapeUnescape::default()),
        Box::new(list_compare::ListComparer::default()),
        Box::new(markdown_preview::MarkdownPreview::default()),
        Box::new(text_compare::TextComparer::default()),
    ]
}
