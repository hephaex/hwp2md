use crate::ir;

pub fn write_markdown(doc: &ir::Document, frontmatter: bool) -> String {
    let mut out = String::new();

    if frontmatter {
        write_frontmatter(&mut out, &doc.metadata);
    }

    for (si, section) in doc.sections.iter().enumerate() {
        if si > 0 {
            out.push_str("\n---\n\n");
        }
        for block in &section.blocks {
            write_block(&mut out, block, 0);
        }
    }

    out
}

fn write_frontmatter(out: &mut String, meta: &ir::Metadata) {
    out.push_str("---\n");
    if let Some(ref title) = meta.title {
        out.push_str(&format!("title: \"{}\"\n", escape_yaml(title)));
    }
    if let Some(ref author) = meta.author {
        out.push_str(&format!("author: \"{}\"\n", escape_yaml(author)));
    }
    if let Some(ref created) = meta.created {
        out.push_str(&format!("date: \"{created}\"\n"));
    }
    if let Some(ref subject) = meta.subject {
        out.push_str(&format!("subject: \"{}\"\n", escape_yaml(subject)));
    }
    if let Some(ref desc) = meta.description {
        out.push_str(&format!("description: \"{}\"\n", escape_yaml(desc)));
    }
    if !meta.keywords.is_empty() {
        let escaped: Vec<String> = meta.keywords.iter().map(|k| escape_yaml(k)).collect();
        out.push_str(&format!("keywords: [{}]\n", escaped.join(", ")));
    }
    out.push_str("---\n\n");
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn write_block(out: &mut String, block: &ir::Block, indent: usize) {
    let prefix: String = "  ".repeat(indent);

    match block {
        ir::Block::Heading { level, inlines } => {
            let hashes = "#".repeat(*level as usize);
            out.push_str(&format!("{hashes} {}\n\n", render_inlines(inlines)));
        }
        ir::Block::Paragraph { inlines } => {
            let text = render_inlines(inlines);
            if !text.trim().is_empty() {
                out.push_str(&format!("{prefix}{text}\n\n"));
            }
        }
        ir::Block::Table { rows, col_count } => {
            write_table(out, rows, *col_count);
        }
        ir::Block::CodeBlock { language, code } => {
            let lang = language.as_deref().unwrap_or("");
            out.push_str(&format!("{prefix}```{lang}\n"));
            for line in code.lines() {
                out.push_str(&format!("{prefix}{line}\n"));
            }
            out.push_str(&format!("{prefix}```\n\n"));
        }
        ir::Block::BlockQuote { blocks } => {
            for b in blocks {
                let mut inner = String::new();
                write_block(&mut inner, b, 0);
                for line in inner.lines() {
                    out.push_str(&format!("{prefix}> {line}\n"));
                }
            }
            out.push('\n');
        }
        ir::Block::List {
            ordered,
            start,
            items,
        } => {
            write_list(out, items, *ordered, *start, indent);
            out.push('\n');
        }
        ir::Block::Image { src, alt } => {
            out.push_str(&format!("{prefix}![{alt}]({src})\n\n"));
        }
        ir::Block::HorizontalRule => {
            out.push_str(&format!("{prefix}---\n\n"));
        }
        ir::Block::Footnote { id, content } => {
            out.push_str(&format!("{prefix}[^{id}]: "));
            for (i, b) in content.iter().enumerate() {
                if i > 0 {
                    out.push_str(&format!("{prefix}    "));
                }
                let mut inner = String::new();
                write_block(&mut inner, b, 0);
                out.push_str(inner.trim_end());
                out.push('\n');
            }
            out.push('\n');
        }
        ir::Block::Math { display, tex } => {
            if *display {
                out.push_str(&format!("{prefix}$$\n{prefix}{tex}\n{prefix}$$\n\n"));
            } else {
                out.push_str(&format!("{prefix}${tex}$"));
            }
        }
    }
}

fn render_inlines(inlines: &[ir::Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        let mut text = inline.text.clone();

        if inline.code {
            out.push_str(&format!("`{text}`"));
            continue;
        }

        if inline.bold && inline.italic {
            text = format!("***{text}***");
        } else if inline.bold {
            text = format!("**{text}**");
        } else if inline.italic {
            text = format!("*{text}*");
        }

        if inline.strikethrough {
            text = format!("~~{text}~~");
        }
        if inline.underline {
            text = format!("<u>{text}</u>");
        }
        if inline.superscript {
            text = format!("<sup>{text}</sup>");
        }
        if inline.subscript {
            text = format!("<sub>{text}</sub>");
        }

        if let Some(ref url) = inline.link {
            text = format!("[{text}]({url})");
        }
        if let Some(ref id) = inline.footnote_ref {
            text = format!("{text}[^{id}]");
        }

        out.push_str(&text);
    }
    out
}

fn write_table(out: &mut String, rows: &[ir::TableRow], col_count: usize) {
    if rows.is_empty() {
        return;
    }

    let has_complex = rows
        .iter()
        .any(|r| r.cells.iter().any(|c| c.colspan > 1 || c.rowspan > 1));

    if has_complex {
        write_html_table(out, rows);
        return;
    }

    let cols = col_count.max(rows.iter().map(|r| r.cells.len()).max().unwrap_or(0));

    for (ri, row) in rows.iter().enumerate() {
        out.push('|');
        for ci in 0..cols {
            let cell_text = if ci < row.cells.len() {
                cell_to_text(&row.cells[ci])
            } else {
                String::new()
            };
            out.push_str(&format!(" {} |", cell_text));
        }
        out.push('\n');

        if ri == 0 {
            out.push('|');
            for _ in 0..cols {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

fn write_html_table(out: &mut String, rows: &[ir::TableRow]) {
    out.push_str("<table>\n");
    for (ri, row) in rows.iter().enumerate() {
        out.push_str("  <tr>\n");
        let tag = if ri == 0 { "th" } else { "td" };
        for cell in &row.cells {
            let mut attrs = String::new();
            if cell.colspan > 1 {
                attrs.push_str(&format!(" colspan=\"{}\"", cell.colspan));
            }
            if cell.rowspan > 1 {
                attrs.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
            }
            let text = cell_to_text(cell);
            out.push_str(&format!("    <{tag}{attrs}>{text}</{tag}>\n"));
        }
        out.push_str("  </tr>\n");
    }
    out.push_str("</table>\n\n");
}

fn cell_to_text(cell: &ir::TableCell) -> String {
    let mut texts = Vec::new();
    for block in &cell.blocks {
        match block {
            ir::Block::Paragraph { inlines } => {
                texts.push(render_inlines(inlines));
            }
            _ => {
                let mut s = String::new();
                write_block(&mut s, block, 0);
                texts.push(s.trim().to_string());
            }
        }
    }
    texts.join(" ").replace('|', "\\|")
}

fn write_list(out: &mut String, items: &[ir::ListItem], ordered: bool, start: u32, indent: usize) {
    let prefix_str: String = "  ".repeat(indent);
    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}.", start as usize + i)
        } else {
            "-".to_string()
        };

        for (bi, block) in item.blocks.iter().enumerate() {
            let mut inner = String::new();
            write_block(&mut inner, block, 0);
            let inner = inner.trim_end();
            if bi == 0 {
                out.push_str(&format!("{prefix_str}{marker} {inner}\n"));
            } else {
                let cont_indent = " ".repeat(marker.len() + 1);
                out.push_str(&format!("{prefix_str}{cont_indent}{inner}\n"));
            }
        }

        if !item.children.is_empty() {
            write_list(out, &item.children, ordered, 1, indent + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn plain(t: &str) -> ir::Inline {
        ir::Inline::plain(t)
    }

    fn make_doc_with_blocks(blocks: Vec<ir::Block>) -> ir::Document {
        let mut doc = ir::Document::new();
        doc.sections.push(ir::Section { blocks });
        doc
    }

    // -----------------------------------------------------------------------
    // escape_yaml
    // -----------------------------------------------------------------------

    #[test]
    fn escape_yaml_backslash() {
        assert_eq!(escape_yaml("a\\b"), "a\\\\b");
    }

    #[test]
    fn escape_yaml_double_quote() {
        assert_eq!(escape_yaml("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn escape_yaml_no_special_chars() {
        assert_eq!(escape_yaml("hello world"), "hello world");
    }

    #[test]
    fn escape_yaml_newline() {
        assert_eq!(escape_yaml("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn escape_yaml_carriage_return() {
        assert_eq!(escape_yaml("a\rb"), "a\\rb");
    }

    #[test]
    fn escape_yaml_tab() {
        assert_eq!(escape_yaml("a\tb"), "a\\tb");
    }

    // -----------------------------------------------------------------------
    // render_inlines
    // -----------------------------------------------------------------------

    #[test]
    fn render_inlines_plain() {
        assert_eq!(render_inlines(&[plain("hello")]), "hello");
    }

    #[test]
    fn render_inlines_bold() {
        let inlines = vec![ir::Inline {
            text: "bold".into(),
            bold: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "**bold**");
    }

    #[test]
    fn render_inlines_italic() {
        let inlines = vec![ir::Inline {
            text: "em".into(),
            italic: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "*em*");
    }

    #[test]
    fn render_inlines_bold_italic() {
        let inlines = vec![ir::Inline {
            text: "bi".into(),
            bold: true,
            italic: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "***bi***");
    }

    #[test]
    fn render_inlines_strikethrough() {
        let inlines = vec![ir::Inline {
            text: "del".into(),
            strikethrough: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "~~del~~");
    }

    #[test]
    fn render_inlines_underline() {
        let inlines = vec![ir::Inline {
            text: "ul".into(),
            underline: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "<u>ul</u>");
    }

    #[test]
    fn render_inlines_superscript() {
        let inlines = vec![ir::Inline {
            text: "sup".into(),
            superscript: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "<sup>sup</sup>");
    }

    #[test]
    fn render_inlines_subscript() {
        let inlines = vec![ir::Inline {
            text: "sub".into(),
            subscript: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "<sub>sub</sub>");
    }

    #[test]
    fn render_inlines_code() {
        let inlines = vec![ir::Inline {
            text: "code()".into(),
            code: true,
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "`code()`");
    }

    #[test]
    fn render_inlines_link() {
        let inlines = vec![ir::Inline {
            text: "click".into(),
            link: Some("https://example.com".into()),
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "[click](https://example.com)");
    }

    #[test]
    fn render_inlines_footnote_ref() {
        let inlines = vec![ir::Inline {
            text: String::new(),
            footnote_ref: Some("1".into()),
            ..Default::default()
        }];
        assert_eq!(render_inlines(&inlines), "[^1]");
    }

    // -----------------------------------------------------------------------
    // write_markdown — block types
    // -----------------------------------------------------------------------

    #[test]
    fn write_markdown_heading_levels() {
        for level in 1u8..=6 {
            let doc = make_doc_with_blocks(vec![ir::Block::Heading {
                level,
                inlines: vec![plain("Title")],
            }]);
            let md = write_markdown(&doc, false);
            let hashes = "#".repeat(level as usize);
            assert!(
                md.starts_with(&format!("{hashes} Title")),
                "level {level}: got {md:?}"
            );
        }
    }

    #[test]
    fn write_markdown_paragraph() {
        let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
            inlines: vec![plain("Hello, world.")],
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("Hello, world."));
    }

    #[test]
    fn write_markdown_code_block() {
        let doc = make_doc_with_blocks(vec![ir::Block::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into(),
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("```rust\n"), "got: {md}");
        assert!(md.contains("fn main() {}"), "got: {md}");
        assert!(md.contains("\n```"), "got: {md}");
    }

    #[test]
    fn write_markdown_code_block_no_language() {
        let doc = make_doc_with_blocks(vec![ir::Block::CodeBlock {
            language: None,
            code: "raw code".into(),
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("```\n"), "got: {md}");
        assert!(md.contains("raw code"), "got: {md}");
    }

    #[test]
    fn write_markdown_simple_gfm_table() {
        let rows = vec![
            ir::TableRow {
                cells: vec![
                    ir::TableCell {
                        blocks: vec![ir::Block::Paragraph {
                            inlines: vec![plain("Name")],
                        }],
                        ..Default::default()
                    },
                    ir::TableCell {
                        blocks: vec![ir::Block::Paragraph {
                            inlines: vec![plain("Age")],
                        }],
                        ..Default::default()
                    },
                ],
                is_header: true,
            },
            ir::TableRow {
                cells: vec![
                    ir::TableCell {
                        blocks: vec![ir::Block::Paragraph {
                            inlines: vec![plain("Alice")],
                        }],
                        ..Default::default()
                    },
                    ir::TableCell {
                        blocks: vec![ir::Block::Paragraph {
                            inlines: vec![plain("30")],
                        }],
                        ..Default::default()
                    },
                ],
                is_header: false,
            },
        ];
        let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("| Name | Age |"), "got: {md}");
        assert!(md.contains("| --- |"), "got: {md}");
        assert!(md.contains("| Alice | 30 |"), "got: {md}");
    }

    #[test]
    fn write_markdown_complex_table_html_fallback() {
        // A cell with colspan > 1 must trigger the HTML table fallback.
        let rows = vec![ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("wide")],
                }],
                colspan: 2,
                rowspan: 1,
            }],
            is_header: true,
        }];
        let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("<table>"), "got: {md}");
        assert!(md.contains("colspan=\"2\""), "got: {md}");
    }

    #[test]
    fn write_markdown_unordered_list() {
        let doc = make_doc_with_blocks(vec![ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![
                ir::ListItem {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("alpha")],
                    }],
                    children: Vec::new(),
                },
                ir::ListItem {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("beta")],
                    }],
                    children: Vec::new(),
                },
            ],
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("- alpha"), "got: {md}");
        assert!(md.contains("- beta"), "got: {md}");
    }

    #[test]
    fn write_markdown_ordered_list() {
        let doc = make_doc_with_blocks(vec![ir::Block::List {
            ordered: true,
            start: 1,
            items: vec![
                ir::ListItem {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("first")],
                    }],
                    children: Vec::new(),
                },
                ir::ListItem {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("second")],
                    }],
                    children: Vec::new(),
                },
            ],
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("1. first"), "got: {md}");
        assert!(md.contains("2. second"), "got: {md}");
    }

    #[test]
    fn write_markdown_image() {
        let doc = make_doc_with_blocks(vec![ir::Block::Image {
            src: "img.png".into(),
            alt: "a picture".into(),
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("![a picture](img.png)"), "got: {md}");
    }

    #[test]
    fn write_markdown_horizontal_rule() {
        let doc = make_doc_with_blocks(vec![ir::Block::HorizontalRule]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("---"), "got: {md}");
    }

    #[test]
    fn write_markdown_math_display() {
        let doc = make_doc_with_blocks(vec![ir::Block::Math {
            display: true,
            tex: "E=mc^2".into(),
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("$$\n"), "got: {md}");
        assert!(md.contains("E=mc^2"), "got: {md}");
    }

    #[test]
    fn write_markdown_math_inline() {
        let doc = make_doc_with_blocks(vec![ir::Block::Math {
            display: false,
            tex: "x+y".into(),
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("$x+y$"), "got: {md}");
    }

    #[test]
    fn write_markdown_footnote() {
        let doc = make_doc_with_blocks(vec![ir::Block::Footnote {
            id: "fn1".into(),
            content: vec![ir::Block::Paragraph {
                inlines: vec![plain("footnote text")],
            }],
        }]);
        let md = write_markdown(&doc, false);
        assert!(md.contains("[^fn1]:"), "got: {md}");
        assert!(md.contains("footnote text"), "got: {md}");
    }

    #[test]
    fn write_markdown_frontmatter() {
        let mut doc = ir::Document::new();
        doc.metadata.title = Some("My Title".into());
        doc.metadata.author = Some("Author Name".into());
        doc.sections.push(ir::Section { blocks: Vec::new() });
        let md = write_markdown(&doc, true);
        assert!(md.starts_with("---\n"), "got: {md}");
        assert!(md.contains("title: \"My Title\""), "got: {md}");
        assert!(md.contains("author: \"Author Name\""), "got: {md}");
    }

    #[test]
    fn write_markdown_multi_section_separator() {
        let mut doc = ir::Document::new();
        doc.sections.push(ir::Section {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("Section 1")],
            }],
        });
        doc.sections.push(ir::Section {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("Section 2")],
            }],
        });
        let md = write_markdown(&doc, false);
        assert!(md.contains("\n---\n"), "got: {md}");
        assert!(md.contains("Section 1"), "got: {md}");
        assert!(md.contains("Section 2"), "got: {md}");
    }
}
