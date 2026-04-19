//! JSON serialization: 1:1 and flattened (dot-notation) variants.
//!
//! Strategy: we serialize the `Node` tree directly through `serde_json::Serializer`
//! into a pre-allocated `String` buffer. For `to_flatten_json` we walk the tree
//! once and emit `(key, value)` pairs to a single JSON object, using a reusable
//! `String` key buffer (`push_str`/`truncate`) to avoid per-level allocations.

use serde::ser::{SerializeMap, Serializer};

use crate::error::Result;
use crate::node::{Children, Node};

/// Produce a 1:1 JSON representation preserving XML structure.
///
/// The output is a single JSON object `{ "<root-tag>": <body> }` where each
/// element becomes either:
/// - a JSON string (pure-text leaf), or
/// - a JSON object with attribute entries (`@name`), text under `#text`,
///   and one key per distinct child tag (array if repeated).
pub fn to_json(root_tag: &str, root: &Node) -> Result<String> {
    let mut buf = Vec::with_capacity(256);
    {
        let mut ser = serde_json::Serializer::new(&mut buf);
        let mut map = ser
            .serialize_map(Some(1))
            .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
        map.serialize_entry(root_tag, &NodeView(root))
            .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
        map.end()
            .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
    }
    Ok(String::from_utf8(buf).expect("serde_json produces valid UTF-8"))
}

/// Produce a flat JSON object using `separator` to join nested element tags.
/// Repeated siblings get `[i]` index suffixes; attributes get `@name`;
/// mixed-content text uses `#text`.
pub fn to_flatten_json(root_tag: &str, root: &Node, separator: &str) -> Result<String> {
    let mut buf = Vec::with_capacity(256);
    let mut key = String::with_capacity(64);
    key.push_str(root_tag);

    {
        let mut ser = serde_json::Serializer::new(&mut buf);
        let mut map = ser
            .serialize_map(None)
            .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
        write_flat(&mut map, &mut key, root, separator)?;
        map.end()
            .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
    }
    Ok(String::from_utf8(buf).expect("serde_json produces valid UTF-8"))
}

/// Wrapper implementing `serde::Serialize` for a `Node` with 1:1 semantics.
struct NodeView<'a>(&'a Node);

impl serde::Serialize for NodeView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self.0 {
            Node::Text(t) => serializer.serialize_str(t),
            Node::Element { attrs, children } => {
                // Pure-text leaf: collapse to the bare string value.
                if let Some(t) = self.0.pure_text() {
                    return serializer.serialize_str(t);
                }
                // Empty element: emit an empty object.
                let entries = attrs.len() + children.len();
                let mut map = serializer.serialize_map(Some(entries))?;
                for (k, v) in attrs {
                    map.serialize_entry(k.as_ref(), v.as_ref())?;
                }
                for (tag, kids) in children {
                    match kids {
                        Children::One(n) => {
                            map.serialize_entry(tag.as_ref(), &NodeView(n))?;
                        }
                        Children::Many(v) => {
                            let list: Vec<NodeView<'_>> = v.iter().map(NodeView).collect();
                            map.serialize_entry(tag.as_ref(), &list)?;
                        }
                    }
                }
                map.end()
            }
        }
    }
}

/// Recursively walk the tree and emit flat `(key, value)` pairs. The `key`
/// buffer is reused across recursion levels.
fn write_flat<M: SerializeMap>(
    map: &mut M,
    key: &mut String,
    node: &Node,
    sep: &str,
) -> Result<()> {
    match node {
        Node::Text(t) => {
            map.serialize_entry(&*key, t.as_ref())
                .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
        }
        Node::Element { attrs, children } => {
            // Pure-text leaf collapses to a string at the current key.
            if let Some(t) = node.pure_text() {
                map.serialize_entry(&*key, t)
                    .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
                return Ok(());
            }
            // Empty element: emit an empty-string value to avoid losing the key.
            if attrs.is_empty() && children.is_empty() {
                map.serialize_entry(&*key, "")
                    .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
                return Ok(());
            }
            let base_len = key.len();
            for (ak, av) in attrs {
                key.push_str(sep);
                key.push_str(ak);
                map.serialize_entry(&*key, av.as_ref())
                    .map_err(|e| crate::error::FlattenerError::Invalid(e.to_string()))?;
                key.truncate(base_len);
            }
            for (tag, kids) in children {
                match kids {
                    Children::One(n) => {
                        key.push_str(sep);
                        key.push_str(tag);
                        write_flat(map, key, n, sep)?;
                        key.truncate(base_len);
                    }
                    Children::Many(v) => {
                        for (i, n) in v.iter().enumerate() {
                            use std::fmt::Write;
                            key.push_str(sep);
                            key.push_str(tag);
                            let _ = write!(key, "[{i}]");
                            write_flat(map, key, n, sep)?;
                            key.truncate(base_len);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn pure_text_leaf_collapses() {
        let (tag, node) = parse("<a>hi</a>").unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert_eq!(out, r#"{"a":"hi"}"#);
    }

    #[test]
    fn nested_1to1() {
        let (tag, node) = parse("<r><a>1</a><b>2</b></r>").unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert_eq!(out, r#"{"r":{"a":"1","b":"2"}}"#);
    }

    #[test]
    fn repeated_is_array() {
        let (tag, node) = parse("<r><i>1</i><i>2</i></r>").unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert_eq!(out, r#"{"r":{"i":["1","2"]}}"#);
    }

    #[test]
    fn flat_dot_notation() {
        let (tag, node) = parse("<r><a><b>x</b></a></r>").unwrap();
        let out = to_flatten_json(&tag, &node, ".").unwrap();
        assert_eq!(out, r#"{"r.a.b":"x"}"#);
    }

    #[test]
    fn flat_array_indexing() {
        let (tag, node) = parse("<r><i>1</i><i>2</i></r>").unwrap();
        let out = to_flatten_json(&tag, &node, ".").unwrap();
        assert_eq!(out, r#"{"r.i[0]":"1","r.i[1]":"2"}"#);
    }

    #[test]
    fn flat_custom_separator() {
        let (tag, node) = parse("<r><a><b>x</b></a></r>").unwrap();
        let out = to_flatten_json(&tag, &node, "_").unwrap();
        assert_eq!(out, r#"{"r_a_b":"x"}"#);
    }

    #[test]
    fn element_with_attrs_1to1() {
        let (tag, node) = parse(r#"<r id="1"><a>x</a></r>"#).unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert!(out.contains(r#""@id":"1""#));
        assert!(out.contains(r#""a":"x""#));
    }

    #[test]
    fn empty_element_1to1() {
        let (tag, node) = parse("<br/>").unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert_eq!(out, r#"{"br":{}}"#);
    }

    #[test]
    fn empty_element_flat() {
        let (tag, node) = parse("<br/>").unwrap();
        let out = to_flatten_json(&tag, &node, ".").unwrap();
        assert_eq!(out, r#"{"br":""}"#);
    }

    #[test]
    fn flat_with_attrs() {
        let (tag, node) = parse(r#"<r id="1"><a>x</a></r>"#).unwrap();
        let out = to_flatten_json(&tag, &node, ".").unwrap();
        assert!(out.contains(r#""r.@id":"1""#));
        assert!(out.contains(r#""r.a":"x""#));
    }

    #[test]
    fn mixed_content_1to1() {
        let (tag, node) = parse(r#"<r id="1">hello</r>"#).unwrap();
        let out = to_json(&tag, &node).unwrap();
        assert!(out.contains(r#""@id":"1""#));
        assert!(out.contains(r##""#text":"hello""##));
    }
}
