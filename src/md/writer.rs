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
        out.push_str(&format!("keywords: [{}]\n", meta.keywords.join(", ")));
    }
    out.push_str("---\n\n");
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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

    let has_complex = rows.iter().any(|r| {
        r.cells
            .iter()
            .any(|c| c.colspan > 1 || c.rowspan > 1)
    });

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

fn write_list(
    out: &mut String,
    items: &[ir::ListItem],
    ordered: bool,
    start: u32,
    indent: usize,
) {
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
