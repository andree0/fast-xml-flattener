//! Record extraction for tabular outputs (CSV and Parquet).
//!
//! A "record" corresponds to one row. We choose the record level
//! automatically:
//! - If the root element has a single repeated-tag child group
//!   (e.g. `<root><user>..</user><user>..</user></root>`), each occurrence
//!   of that child becomes one row.
//! - Otherwise, the root element itself is a single row.
//!
//! Each row is produced as `IndexMap<String, String>` — column order is the
//! order of first appearance across all rows.

use indexmap::IndexMap;

use crate::node::{Children, Node};

/// Produce (columns-in-first-appearance-order, rows) for a parsed document.
///
/// `include_attrs` — when false, attribute entries (keys starting with '@')
/// are omitted from both column set and row values.
/// `separator` — used to join nested element tags into dotted column names.
pub fn extract_records(
    root_tag: &str,
    root: &Node,
    separator: &str,
    include_attrs: bool,
) -> (Vec<String>, Vec<IndexMap<String, String>>) {
    let (record_nodes, record_prefix) = select_record_nodes(root_tag, root);

    let mut columns: IndexMap<String, ()> = IndexMap::new();
    let mut rows: Vec<IndexMap<String, String>> = Vec::with_capacity(record_nodes.len());

    for rec in record_nodes {
        let mut row: IndexMap<String, String> = IndexMap::new();
        let mut key = String::with_capacity(32);
        key.push_str(&record_prefix);
        flatten(rec, &mut key, separator, include_attrs, &mut row);
        for col in row.keys() {
            if !columns.contains_key(col) {
                columns.insert(col.clone(), ());
            }
        }
        rows.push(row);
    }

    let cols: Vec<String> = columns.into_keys().collect();
    (cols, rows)
}

/// Decide which node(s) should be treated as records.
/// - If root has exactly one child group, and that group is `Many`, each
///   `Many` element is a record. The prefix becomes that child's tag.
/// - Otherwise, the root itself is the single record.
fn select_record_nodes<'a>(root_tag: &str, root: &'a Node) -> (Vec<&'a Node>, String) {
    if let Node::Element {
        children, attrs, ..
    } = root
    {
        if attrs.is_empty() && children.len() == 1 {
            let (_, kids) = children.iter().next().unwrap();
            if let Children::Many(v) = kids {
                return (v.iter().collect(), String::new());
            }
        }
    }
    (vec![root], root_tag.to_string())
}

/// Flatten `node` into `row` using dotted keys (joined by `separator`).
/// Attributes keep their `@` prefix; mixed text uses `#text`; pure-text
/// leaves collapse to their string value at the current key.
fn flatten(
    node: &Node,
    key: &mut String,
    sep: &str,
    include_attrs: bool,
    row: &mut IndexMap<String, String>,
) {
    match node {
        Node::Text(t) => {
            row.insert(key.clone(), t.as_ref().to_owned());
        }
        Node::Element { attrs, children } => {
            if let Some(t) = node.pure_text() {
                if !key.is_empty() {
                    row.insert(key.clone(), t.to_owned());
                }
                return;
            }
            if attrs.is_empty() && children.is_empty() && !key.is_empty() {
                row.insert(key.clone(), String::new());
                return;
            }
            let base_len = key.len();
            if include_attrs {
                for (ak, av) in attrs {
                    push_key(key, sep, ak);
                    row.insert(key.clone(), av.as_ref().to_owned());
                    key.truncate(base_len);
                }
            }
            for (tag, kids) in children {
                match kids {
                    Children::One(n) => {
                        push_key(key, sep, tag);
                        flatten(n, key, sep, include_attrs, row);
                        key.truncate(base_len);
                    }
                    Children::Many(v) => {
                        for (i, n) in v.iter().enumerate() {
                            use std::fmt::Write;
                            push_key(key, sep, tag);
                            let _ = write!(key, "[{i}]");
                            flatten(n, key, sep, include_attrs, row);
                            key.truncate(base_len);
                        }
                    }
                }
            }
        }
    }
}

fn push_key(key: &mut String, sep: &str, part: &str) {
    if !key.is_empty() {
        key.push_str(sep);
    }
    key.push_str(part);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn single_record_root() {
        let (tag, node) = parse("<x><a>1</a><b>2</b></x>").unwrap();
        let (cols, rows) = extract_records(&tag, &node, ".", true);
        assert_eq!(cols, vec!["x.a", "x.b"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("x.a").map(String::as_str), Some("1"));
    }

    #[test]
    fn multi_record_detection() {
        let (tag, node) = parse("<xs><x><a>1</a></x><x><a>2</a></x></xs>").unwrap();
        let (cols, rows) = extract_records(&tag, &node, ".", true);
        assert_eq!(cols, vec!["a"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("a").map(String::as_str), Some("1"));
        assert_eq!(rows[1].get("a").map(String::as_str), Some("2"));
    }

    #[test]
    fn include_attrs_flag() {
        let (tag, node) = parse(r#"<x a="1"><b>2</b></x>"#).unwrap();
        let (cols_with, _) = extract_records(&tag, &node, ".", true);
        assert!(cols_with.iter().any(|c| c == "x.@a"));
        let (cols_without, _) = extract_records(&tag, &node, ".", false);
        assert!(!cols_without.iter().any(|c| c.contains('@')));
    }
}
