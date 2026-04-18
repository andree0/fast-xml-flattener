//! XML parser: converts XML (from a string or file) into the internal `Node`
//! tree using `quick-xml`'s streaming reader.
//!
//! Performance notes:
//! - `quick-xml` is a pull parser, so no DOM is built by the library itself.
//! - We drive events into a single growing `Node` tree. Stack depth is bounded
//!   by XML nesting depth, not document size.
//! - Whitespace-only text between elements is dropped to avoid polluting
//!   the tree with empty `#text` nodes. Whitespace inside explicit text
//!   (or between sibling text fragments) is preserved.
//! - Namespaces are stripped from tag names (local-name only). Attribute
//!   namespaces are likewise stripped. This matches xmltodict behavior.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Decoder;
use quick_xml::Reader;

use crate::error::{FlattenerError, Result};
use crate::node::{Children, Node, TEXT_KEY};

/// Parse an XML string into a single root `Node::Element`.
pub fn parse(xml: &str) -> Result<(Box<str>, Node)> {
    let mut reader = Reader::from_str(xml);
    configure(&mut reader);
    parse_reader(&mut reader)
}

/// Parse an XML file into a single root `Node::Element`.
/// The file is read in buffered chunks — the full content is never held in
/// memory at once.
pub fn parse_file(path: &Path) -> Result<(Box<str>, Node)> {
    let file = File::open(path).map_err(FlattenerError::Io)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    configure(&mut reader);
    parse_reader(&mut reader)
}

/// Apply shared reader configuration.
fn configure<R: BufRead>(reader: &mut Reader<R>) {
    let cfg = reader.config_mut();
    cfg.trim_text(false);
    cfg.expand_empty_elements = false;
}

/// Drive the event loop for any `BufRead`-backed reader and build the Node
/// tree. Stack depth is bounded by XML nesting depth, not document size.
fn parse_reader<R: BufRead>(reader: &mut Reader<R>) -> Result<(Box<str>, Node)> {
    // `Decoder` is Copy — safe to capture before the mutable borrow loop.
    let decoder = reader.decoder();

    let mut stack: Vec<(Box<str>, Node)> = Vec::with_capacity(16);
    let mut root: Option<(Box<str>, Node)> = None;
    let mut buf = Vec::with_capacity(256);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let (tag, node) = start_element(e, decoder)?;
                stack.push((tag, node));
            }
            Event::Empty(ref e) => {
                let (tag, node) = start_element(e, decoder)?;
                attach(&mut stack, &mut root, tag, node);
            }
            Event::End(_) => {
                let (tag, node) = stack
                    .pop()
                    .ok_or_else(|| FlattenerError::Xml("unbalanced end tag".into()))?;
                attach(&mut stack, &mut root, tag, node);
            }
            Event::Text(ref e) => {
                // Decode UTF-8 first, then resolve any entity references that
                // may appear within the text slice.
                let decoded = e.decode()?;
                let unescaped = quick_xml::escape::unescape(&decoded)?;
                let text = unescaped.as_ref();
                if !text.trim().is_empty() {
                    push_text(&mut stack, text)?;
                }
            }
            Event::GeneralRef(ref e) => {
                // quick-xml 0.39 emits general entity references (`&amp;`,
                // `&lt;`, numeric char refs, ...) as their own event. Resolve
                // by asking `unescape` to expand the `&name;` form.
                let name = std::str::from_utf8(e.as_ref())?;
                let with_markers = format!("&{name};");
                let resolved = quick_xml::escape::unescape(&with_markers)?;
                push_text(&mut stack, resolved.as_ref())?;
            }
            Event::CData(ref e) => {
                let text = std::str::from_utf8(e.as_ref())?;
                push_text(&mut stack, text)?;
            }
            Event::Eof => break,
            // Comments, declarations, processing instructions, doctype: ignored.
            _ => {}
        }
        buf.clear();
    }

    root.ok_or_else(|| FlattenerError::Invalid("empty XML document".into()))
}

/// Build an `Element` node from a start tag, extracting its attributes.
/// Namespace declarations (`xmlns`, `xmlns:*`) are skipped — they control
/// parsing, not the document's data.
fn start_element(e: &BytesStart<'_>, decoder: Decoder) -> Result<(Box<str>, Node)> {
    let tag = local_name_to_string(e.name())?;
    let mut node = Node::empty_element();
    if let Node::Element { attrs, .. } = &mut node {
        for attr in e.attributes() {
            let attr = attr?;
            let raw_key = attr.key.as_ref();
            if raw_key == b"xmlns" || raw_key.starts_with(b"xmlns:") {
                continue;
            }
            let key = local_name_to_string(attr.key)?;
            let key_with_prefix = format!("@{key}").into_boxed_str();
            let value = attr.decode_and_unescape_value(decoder)?;
            attrs.insert(key_with_prefix, value.into_owned().into_boxed_str());
        }
    }
    Ok((tag.into_boxed_str(), node))
}

/// Append `text` to the current element's `#text` child. Multiple text
/// fragments accumulate in a single `#text` node (e.g. text split around
/// a CDATA section is concatenated).
fn push_text(stack: &mut [(Box<str>, Node)], text: &str) -> Result<()> {
    let (_, current) = stack
        .last_mut()
        .ok_or_else(|| FlattenerError::Xml("text outside root element".into()))?;

    if let Node::Element { children, .. } = current {
        match children.shift_remove(TEXT_KEY) {
            Some(Children::One(prev)) => {
                if let Node::Text(prev_text) = *prev {
                    let mut combined = String::with_capacity(prev_text.len() + text.len());
                    combined.push_str(&prev_text);
                    combined.push_str(text);
                    children.insert(
                        TEXT_KEY.into(),
                        Children::One(Box::new(Node::Text(combined.into_boxed_str()))),
                    );
                } else {
                    children.insert(
                        TEXT_KEY.into(),
                        Children::One(Box::new(Node::Text(text.into()))),
                    );
                }
            }
            _ => {
                children.insert(
                    TEXT_KEY.into(),
                    Children::One(Box::new(Node::Text(text.into()))),
                );
            }
        }
    }
    Ok(())
}

/// Attach a completed (`tag`, `node`) either to the current parent (if the
/// stack is non-empty) or promote it to the document root.
fn attach(
    stack: &mut [(Box<str>, Node)],
    root: &mut Option<(Box<str>, Node)>,
    tag: Box<str>,
    node: Node,
) {
    if let Some((_, parent)) = stack.last_mut() {
        parent.insert_child(tag, node);
    } else {
        *root = Some((tag, node));
    }
}

/// Strip any namespace prefix and return the local part of the QName as a
/// `String`. quick-xml exposes this via `local_name()`.
fn local_name_to_string(name: QName<'_>) -> Result<String> {
    let local = name.local_name();
    let s = std::str::from_utf8(local.as_ref())?;
    Ok(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_element() {
        let (tag, node) = parse("<a>hello</a>").unwrap();
        assert_eq!(tag.as_ref(), "a");
        assert_eq!(node.pure_text(), Some("hello"));
    }

    #[test]
    fn nested_elements() {
        let (tag, _) = parse("<root><a>1</a><b>2</b></root>").unwrap();
        assert_eq!(tag.as_ref(), "root");
    }

    #[test]
    fn attributes_are_prefixed() {
        let (_, node) = parse(r#"<x a="1" b="2"/>"#).unwrap();
        if let Node::Element { attrs, .. } = node {
            assert_eq!(attrs.get("@a").map(|v| v.as_ref()), Some("1"));
            assert_eq!(attrs.get("@b").map(|v| v.as_ref()), Some("2"));
        } else {
            panic!("expected Element");
        }
    }

    #[test]
    fn repeated_children_become_many() {
        let (_, node) = parse("<r><i>1</i><i>2</i><i>3</i></r>").unwrap();
        if let Node::Element { children, .. } = node {
            match children.get("i") {
                Some(Children::Many(v)) => assert_eq!(v.len(), 3),
                _ => panic!("expected Many"),
            }
        }
    }

    #[test]
    fn whitespace_only_text_is_skipped() {
        let (_, node) = parse("<r>\n  <a>x</a>\n</r>").unwrap();
        if let Node::Element { children, .. } = node {
            assert!(!children.contains_key(TEXT_KEY));
            assert!(children.contains_key("a"));
        }
    }

    #[test]
    fn cdata_is_preserved() {
        let (_, node) = parse("<r><![CDATA[<raw>]]></r>").unwrap();
        assert_eq!(node.pure_text(), Some("<raw>"));
    }

    #[test]
    fn namespaces_are_stripped() {
        let (tag, node) =
            parse(r#"<ns:root xmlns:ns="http://x"><ns:a>1</ns:a></ns:root>"#).unwrap();
        assert_eq!(tag.as_ref(), "root");
        if let Node::Element { children, .. } = node {
            assert!(children.contains_key("a"));
        }
    }

    #[test]
    fn empty_input_errors() {
        assert!(parse("").is_err());
    }

    #[test]
    fn malformed_errors() {
        assert!(parse("<a><b></a>").is_err());
    }
}
