//! XML parser: converts XML (from a string or file) into the internal `Node`
//! tree using `quick-xml`'s streaming reader.
//!
//! Performance notes:
//! - `quick-xml` is a pull parser, so no DOM is built by the library itself.
//! - We drive events into a single growing `Node` tree. Stack depth is bounded
//!   by XML nesting depth, not document size.
//! - With `strip_whitespace=true` (default), whitespace-only text between
//!   elements is dropped and text values are trimmed. With `false`, all text
//!   is preserved verbatim — including `#text` nodes that contain only
//!   whitespace between sibling elements.
//! - With `keep_namespace_declarations=false` (default) namespace declaration
//!   attributes (`xmlns`, `xmlns:*`) are skipped — matches xmltodict default.
//!   With `true` they are kept as attributes with full key name
//!   (`@xmlns`, `@xmlns:tns`).
//! - Tag names always have their namespace prefix stripped (local-name only),
//!   independent of `keep_namespace_declarations`.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Decoder;
use quick_xml::Reader;

use crate::error::{FlattenerError, Result};
use crate::node::{Children, Node, TEXT_KEY};

/// Runtime-tunable parser behavior.
#[derive(Debug, Clone, Copy)]
pub struct ParserConfig {
    /// Trim leading/trailing whitespace from text values and drop
    /// whitespace-only text between elements. Default: `true`.
    pub strip_whitespace: bool,
    /// Preserve `xmlns` / `xmlns:*` attributes as element attributes
    /// (with their full key including `xmlns:` prefix). Default: `false`.
    pub keep_namespace_declarations: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            strip_whitespace: true,
            keep_namespace_declarations: false,
        }
    }
}

/// Parse an XML string into a single root `Node::Element`.
pub fn parse(xml: &str, cfg: &ParserConfig) -> Result<(Box<str>, Node)> {
    let mut reader = Reader::from_str(xml);
    configure(&mut reader);
    parse_reader(&mut reader, cfg)
}

/// Parse an XML file into a single root `Node::Element`.
/// The file is read in buffered chunks — the full content is never held in
/// memory at once.
pub fn parse_file(path: &Path, cfg: &ParserConfig) -> Result<(Box<str>, Node)> {
    let file = File::open(path).map_err(FlattenerError::Io)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    configure(&mut reader);
    parse_reader(&mut reader, cfg)
}

/// Read only the document prologue and return the local name of the first
/// element encountered (the document root). Namespace prefixes are always
/// stripped — the user-facing contract is "root tag without namespace".
pub fn root_tag_name_from_str(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    configure(&mut reader);
    root_tag_name_from_reader(&mut reader)
}

/// File counterpart of [`root_tag_name_from_str`]. Reads only as far as the
/// first start tag.
pub fn root_tag_name_from_file(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(FlattenerError::Io)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    configure(&mut reader);
    root_tag_name_from_reader(&mut reader)
}

/// Apply shared reader configuration.
fn configure<R: BufRead>(reader: &mut Reader<R>) {
    let cfg = reader.config_mut();
    cfg.trim_text(false);
    cfg.expand_empty_elements = false;
}

/// Drive the event loop for any `BufRead`-backed reader and build the Node
/// tree. Stack depth is bounded by XML nesting depth, not document size.
fn parse_reader<R: BufRead>(
    reader: &mut Reader<R>,
    cfg: &ParserConfig,
) -> Result<(Box<str>, Node)> {
    // `Decoder` is Copy — safe to capture before the mutable borrow loop.
    let decoder = reader.decoder();

    let mut stack: Vec<(Box<str>, Node)> = Vec::with_capacity(16);
    let mut root: Option<(Box<str>, Node)> = None;
    let mut buf = Vec::with_capacity(256);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let (tag, node) = start_element(e, decoder, cfg.keep_namespace_declarations)?;
                stack.push((tag, node));
            }
            Event::Empty(ref e) => {
                let (tag, node) = start_element(e, decoder, cfg.keep_namespace_declarations)?;
                attach(&mut stack, &mut root, tag, node);
            }
            Event::End(_) => {
                let (tag, mut node) = stack
                    .pop()
                    .ok_or_else(|| FlattenerError::Xml("unbalanced end tag".into()))?;
                if cfg.strip_whitespace {
                    trim_text_node(&mut node);
                }
                attach(&mut stack, &mut root, tag, node);
            }
            Event::Text(ref e) => {
                // Decode UTF-8 first, then resolve any entity references that
                // may appear within the text slice.
                let decoded = e.decode()?;
                let unescaped = quick_xml::escape::unescape(&decoded)?;
                let text = unescaped.as_ref();
                if cfg.strip_whitespace {
                    // Drop whitespace-only fragments between elements to avoid
                    // creating spurious `#text` nodes. The final value is
                    // trimmed when the parent element ends — this preserves
                    // whitespace around entity refs that come as separate
                    // events (`hello &amp; world` → "hello & world").
                    if !text.trim().is_empty() {
                        push_text(&mut stack, text)?;
                    }
                } else {
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

/// Shared event loop for the lightweight `root_tag_name_*` helpers — stops at
/// the first start tag and returns its local name.
fn root_tag_name_from_reader<R: BufRead>(reader: &mut Reader<R>) -> Result<String> {
    let mut buf = Vec::with_capacity(128);
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) | Event::Empty(ref e) => {
                return local_name_to_string(e.name());
            }
            Event::Eof => {
                return Err(FlattenerError::Invalid("empty XML document".into()));
            }
            _ => {}
        }
        buf.clear();
    }
}

/// Build an `Element` node from a start tag, extracting its attributes.
/// Namespace declarations (`xmlns`, `xmlns:*`) are skipped unless
/// `keep_namespace_declarations` is set — when kept, the full key
/// (e.g. `@xmlns:tns`) is preserved.
fn start_element(
    e: &BytesStart<'_>,
    decoder: Decoder,
    keep_namespace_declarations: bool,
) -> Result<(Box<str>, Node)> {
    let tag = local_name_to_string(e.name())?;
    let mut node = Node::empty_element();
    if let Node::Element { attrs, .. } = &mut node {
        for attr in e.attributes() {
            let attr = attr?;
            let raw_key = attr.key.as_ref();
            let is_ns_decl = raw_key == b"xmlns" || raw_key.starts_with(b"xmlns:");
            if is_ns_decl && !keep_namespace_declarations {
                continue;
            }
            // For namespace declarations we want to preserve the full
            // attribute name (`xmlns`, `xmlns:tns`) so the consumer can
            // recognize it. Other attributes get their local part only.
            let key_str = if is_ns_decl {
                std::str::from_utf8(raw_key)?.to_owned()
            } else {
                local_name_to_string(attr.key)?
            };
            let key_with_prefix = format!("@{key_str}").into_boxed_str();
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

/// Trim the `#text` child of an element (if any). Used when
/// `strip_whitespace=true` to remove leading/trailing whitespace from the
/// element's collected text content. The trim happens once per element,
/// after all text fragments (including those split around entity refs and
/// CDATA) have been concatenated.
fn trim_text_node(node: &mut Node) {
    if let Node::Element { children, .. } = node {
        if let Some(Children::One(boxed)) = children.get_mut(TEXT_KEY) {
            if let Node::Text(t) = boxed.as_mut() {
                let trimmed = t.trim();
                if trimmed.len() != t.len() {
                    *t = trimmed.to_owned().into_boxed_str();
                }
            }
        }
    }
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

    fn dflt() -> ParserConfig {
        ParserConfig::default()
    }

    #[test]
    fn simple_element() {
        let (tag, node) = parse("<a>hello</a>", &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "a");
        assert_eq!(node.pure_text(), Some("hello"));
    }

    #[test]
    fn nested_elements() {
        let (tag, _) = parse("<root><a>1</a><b>2</b></root>", &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "root");
    }

    #[test]
    fn attributes_are_prefixed() {
        let (_, node) = parse(r#"<x a="1" b="2"/>"#, &dflt()).unwrap();
        if let Node::Element { attrs, .. } = node {
            assert_eq!(attrs.get("@a").map(|v| v.as_ref()), Some("1"));
            assert_eq!(attrs.get("@b").map(|v| v.as_ref()), Some("2"));
        } else {
            panic!("expected Element");
        }
    }

    #[test]
    fn repeated_children_become_many() {
        let (_, node) = parse("<r><i>1</i><i>2</i><i>3</i></r>", &dflt()).unwrap();
        if let Node::Element { children, .. } = node {
            match children.get("i") {
                Some(Children::Many(v)) => assert_eq!(v.len(), 3),
                _ => panic!("expected Many"),
            }
        }
    }

    #[test]
    fn whitespace_only_text_is_skipped() {
        let (_, node) = parse("<r>\n  <a>x</a>\n</r>", &dflt()).unwrap();
        if let Node::Element { children, .. } = node {
            assert!(!children.contains_key(TEXT_KEY));
            assert!(children.contains_key("a"));
        }
    }

    #[test]
    fn cdata_is_preserved() {
        let (_, node) = parse("<r><![CDATA[<raw>]]></r>", &dflt()).unwrap();
        assert_eq!(node.pure_text(), Some("<raw>"));
    }

    #[test]
    fn namespaces_are_stripped() {
        let (tag, node) = parse(
            r#"<ns:root xmlns:ns="http://x"><ns:a>1</ns:a></ns:root>"#,
            &dflt(),
        )
        .unwrap();
        assert_eq!(tag.as_ref(), "root");
        if let Node::Element { children, .. } = node {
            assert!(children.contains_key("a"));
        }
    }

    #[test]
    fn empty_input_errors() {
        assert!(parse("", &dflt()).is_err());
    }

    #[test]
    fn malformed_errors() {
        assert!(parse("<a><b></a>", &dflt()).is_err());
    }

    #[test]
    fn entity_reference_expanded() {
        let (_, node) = parse("<r>&amp;</r>", &dflt()).unwrap();
        assert_eq!(node.pure_text(), Some("&"));
    }

    #[test]
    fn unknown_entity_errors() {
        assert!(parse("<r>&undefined_entity_xyz;</r>", &dflt()).is_err());
    }

    #[test]
    fn text_fragments_concatenate() {
        let (_, node) = parse("<r>hello<![CDATA[ world]]></r>", &dflt()).unwrap();
        assert_eq!(node.pure_text(), Some("hello world"));
    }

    #[test]
    fn empty_self_closing_element() {
        let (tag, node) = parse("<br/>", &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "br");
        if let Node::Element { attrs, children } = &node {
            assert!(attrs.is_empty());
            assert!(children.is_empty());
        } else {
            panic!("expected Element");
        }
    }

    #[test]
    fn comments_are_ignored() {
        let (_, node) = parse("<r><!-- comment -->hello</r>", &dflt()).unwrap();
        assert_eq!(node.pure_text(), Some("hello"));
    }

    #[test]
    fn processing_instruction_ignored() {
        let (tag, _) = parse(r#"<?xml version="1.0"?><r>text</r>"#, &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "r");
    }

    #[test]
    fn default_namespace_attribute_skipped() {
        let (_, node) = parse(r#"<r xmlns="http://example.com">text</r>"#, &dflt()).unwrap();
        if let Node::Element { attrs, .. } = &node {
            assert!(!attrs.iter().any(|(k, _)| k.as_ref().contains("xmlns")));
        }
        assert_eq!(node.pure_text(), Some("text"));
    }

    #[test]
    fn parse_file_reads_xml() {
        use std::io::Write;
        let path = std::env::temp_dir().join("fxf_parser_test.xml");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"<root><child>value</child></root>").unwrap();
        }
        let (tag, node) = parse_file(&path, &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "root");
        if let Node::Element { children, .. } = &node {
            assert!(children.contains_key("child"));
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn parse_file_not_found_errors() {
        let path = std::path::Path::new("/nonexistent_path_xyz_fxf/file.xml");
        assert!(parse_file(path, &dflt()).is_err());
    }

    #[test]
    fn deeply_nested_elements() {
        let (tag, _) = parse("<a><b><c><d>deep</d></c></b></a>", &dflt()).unwrap();
        assert_eq!(tag.as_ref(), "a");
    }

    #[test]
    fn element_with_attrs_and_children() {
        let (_, node) = parse(r#"<r id="42"><a>hello</a></r>"#, &dflt()).unwrap();
        if let Node::Element { attrs, children } = &node {
            assert_eq!(attrs.get("@id").map(|v| v.as_ref()), Some("42"));
            assert!(children.contains_key("a"));
        } else {
            panic!("expected Element");
        }
    }

    // --- new tests for ParserConfig options ---

    #[test]
    fn strip_whitespace_true_trims_text_values() {
        let (_, node) = parse("<a>  hello  </a>", &dflt()).unwrap();
        assert_eq!(node.pure_text(), Some("hello"));
    }

    #[test]
    fn strip_whitespace_false_preserves_text() {
        let cfg = ParserConfig {
            strip_whitespace: false,
            ..Default::default()
        };
        let (_, node) = parse("<a>  hello  </a>", &cfg).unwrap();
        assert_eq!(node.pure_text(), Some("  hello  "));
    }

    #[test]
    fn strip_whitespace_false_keeps_whitespace_between_elements() {
        let cfg = ParserConfig {
            strip_whitespace: false,
            ..Default::default()
        };
        let (_, node) = parse("<r>\n  <a>x</a>\n</r>", &cfg).unwrap();
        if let Node::Element { children, .. } = node {
            assert!(children.contains_key(TEXT_KEY));
            assert!(children.contains_key("a"));
        }
    }

    #[test]
    fn keep_namespace_declarations_true_keeps_xmlns_attr() {
        let cfg = ParserConfig {
            keep_namespace_declarations: true,
            ..Default::default()
        };
        let (_, node) = parse(
            r#"<root xmlns:tns="http://ex.com" xmlns="http://def.com"/>"#,
            &cfg,
        )
        .unwrap();
        if let Node::Element { attrs, .. } = node {
            assert_eq!(attrs.get("@xmlns:tns").map(|v| v.as_ref()), Some("http://ex.com"));
            assert_eq!(attrs.get("@xmlns").map(|v| v.as_ref()), Some("http://def.com"));
        } else {
            panic!("expected Element");
        }
    }

    #[test]
    fn keep_namespace_declarations_true_does_not_change_tag_local_name() {
        let cfg = ParserConfig {
            keep_namespace_declarations: true,
            ..Default::default()
        };
        let (tag, _) =
            parse(r#"<tns:root xmlns:tns="http://x"><tns:a>1</tns:a></tns:root>"#, &cfg).unwrap();
        // Tag name is always local-only, regardless of the flag.
        assert_eq!(tag.as_ref(), "root");
    }

    #[test]
    fn root_tag_name_strips_prefix() {
        let name = root_tag_name_from_str(r#"<tns:root xmlns:tns="http://x"/>"#).unwrap();
        assert_eq!(name, "root");
    }

    #[test]
    fn root_tag_name_with_prolog() {
        let name = root_tag_name_from_str(r#"<?xml version="1.0"?><foo/>"#).unwrap();
        assert_eq!(name, "foo");
    }

    #[test]
    fn root_tag_name_empty_errors() {
        assert!(root_tag_name_from_str("").is_err());
    }

    #[test]
    fn root_tag_name_from_file_reads_first_tag() {
        use std::io::Write;
        let path = std::env::temp_dir().join("fxf_root_tag_test.xml");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"<?xml version=\"1.0\"?><a:root xmlns:a=\"http://x\"><a:child/></a:root>")
                .unwrap();
        }
        let name = root_tag_name_from_file(&path).unwrap();
        assert_eq!(name, "root");
        let _ = std::fs::remove_file(&path);
    }
}
