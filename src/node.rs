//! Internal tree representation of a parsed XML document.
//!
//! Design notes:
//! - `Children::One` is used for single-occurrence child tags to avoid a
//!   `Vec` allocation in the common case. When a second sibling with the
//!   same tag is encountered, the variant is promoted to `Children::Many`.
//! - Text content of an element is stored in a reserved child key `#text`
//!   (xmltodict convention). Attribute keys are prefixed with `@`.
//! - `IndexMap` preserves insertion order without a second pass.

use indexmap::IndexMap;

pub const TEXT_KEY: &str = "#text";

/// A node in the parsed XML tree.
#[derive(Debug, Clone)]
pub enum Node {
    /// An XML element with optional attributes and zero or more children.
    Element {
        attrs: IndexMap<Box<str>, Box<str>>,
        children: IndexMap<Box<str>, Children>,
    },
    /// A text leaf (PCDATA or CDATA).
    Text(Box<str>),
}

/// Either a single child or a collection of siblings that share the same tag.
#[derive(Debug, Clone)]
pub enum Children {
    One(Box<Node>),
    Many(Vec<Node>),
}

impl Node {
    /// Create an empty element with no attributes and no children.
    pub fn empty_element() -> Self {
        Node::Element {
            attrs: IndexMap::new(),
            children: IndexMap::new(),
        }
    }

    /// Insert a child under `tag`, promoting to `Children::Many` if needed.
    pub fn insert_child(&mut self, tag: Box<str>, child: Node) {
        if let Node::Element { children, .. } = self {
            match children.shift_remove(&tag) {
                Some(Children::One(prev)) => {
                    children.insert(tag, Children::Many(vec![*prev, child]));
                }
                Some(Children::Many(mut v)) => {
                    v.push(child);
                    children.insert(tag, Children::Many(v));
                }
                None => {
                    children.insert(tag, Children::One(Box::new(child)));
                }
            }
        }
    }

    /// If this is a pure-text leaf (single `#text` child, no attributes),
    /// return its text content. Used by 1:1 outputs to collapse the node
    /// to a bare string value. Returns `None` for mixed content, elements
    /// with attributes, or non-text children.
    pub fn pure_text(&self) -> Option<&str> {
        if let Node::Element { attrs, children } = self {
            if attrs.is_empty() && children.len() == 1 {
                if let Some(Children::One(n)) = children.get(TEXT_KEY) {
                    if let Node::Text(t) = n.as_ref() {
                        return Some(t);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_promotes_to_many() {
        let mut el = Node::empty_element();
        el.insert_child("item".into(), Node::Text("a".into()));
        el.insert_child("item".into(), Node::Text("b".into()));
        if let Node::Element { children, .. } = &el {
            match children.get("item") {
                Some(Children::Many(v)) => assert_eq!(v.len(), 2),
                _ => panic!("expected Many"),
            }
        }
    }

    #[test]
    fn pure_text_leaf_detection() {
        let mut el = Node::empty_element();
        el.insert_child(TEXT_KEY.into(), Node::Text("hello".into()));
        assert_eq!(el.pure_text(), Some("hello"));
    }
}
