use super::*;
use crate::ir::{Asset, Block, Document, Inline, ListItem, Metadata, Section, TableCell, TableRow};
use std::io::Read as _;

use crate::hwpx::read_hwpx;
use crate::hwpx::reader::parse_section_xml;

// ── helpers ──────────────────────────────────────────────────────────────

fn inline(text: &str) -> Inline {
    Inline::plain(text)
}

fn bold_inline(text: &str) -> Inline {
    Inline {
        text: text.into(),
        bold: true,
        ..Inline::default()
    }
}

fn italic_inline(text: &str) -> Inline {
    Inline {
        text: text.into(),
        italic: true,
        ..Inline::default()
    }
}

fn underline_inline(text: &str) -> Inline {
    Inline {
        text: text.into(),
        underline: true,
        ..Inline::default()
    }
}

fn section_xml(blocks: Vec<Block>) -> String {
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: blocks.clone(),

            page_layout: None,
        }],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc);
    let sec = Section {
        blocks,
        page_layout: None,
    };
    let empty_asset_map = ImageAssetMap::new();
    generate_section_xml(&sec, 0, &tables, &empty_asset_map).expect("generate_section_xml failed")
}

fn zip_entry_names(path: &std::path::Path) -> Vec<String> {
    let file = std::fs::File::open(path).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_owned())
        .collect()
}

fn read_hwpx_section_xml(xml: &str) -> Section {
    parse_section_xml(xml).expect("parse_section_xml failed")
}

fn doc_with_section(blocks: Vec<Block>) -> Document {
    Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks,
            page_layout: None,
        }],
        assets: Vec::new(),
    }
}

// ── sub-modules ──────────────────────────────────────────────────────────

#[path = "writer_tests_section.rs"]
mod tests_section;

#[path = "writer_tests_charpr.rs"]
mod tests_charpr;

#[path = "writer_tests_metadata.rs"]
mod tests_metadata;

#[path = "writer_tests_roundtrip.rs"]
mod tests_roundtrip;

#[path = "writer_tests_hyperlink.rs"]
mod tests_hyperlink;

#[path = "writer_tests_ruby.rs"]
mod tests_ruby;

#[path = "writer_tests_footnote.rs"]
mod tests_footnote;

#[path = "writer_tests_image.rs"]
mod tests_image;

#[path = "writer_tests_list.rs"]
mod tests_list;

#[path = "writer_tests_golden.rs"]
mod tests_golden;

#[path = "writer_tests_code_lang.rs"]
mod tests_code_lang;

#[path = "writer_tests_page_layout.rs"]
mod tests_page_layout;

#[path = "writer_tests_para_pr.rs"]
mod tests_para_pr;
