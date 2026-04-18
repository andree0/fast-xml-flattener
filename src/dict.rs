//! Direct `Node` → Python `dict` construction via PyO3.
//!
//! We build `PyDict` / `PyList` objects in a single tree walk, skipping any
//! intermediate JSON/`serde_json::Value` representation. For flattening,
//! a reusable `String` key buffer is pushed/truncated so allocation happens
//! only when capacity actually grows.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::node::{Children, Node};

/// Build a 1:1 nested `PyDict` for the root node.
pub fn to_dict<'py>(py: Python<'py>, root_tag: &str, root: &Node) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item(root_tag, node_to_py(py, root)?)?;
    Ok(out)
}

/// Build a flat `PyDict` using `separator` to join element tags.
pub fn to_flatten_dict<'py>(
    py: Python<'py>,
    root_tag: &str,
    root: &Node,
    separator: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    let mut key = String::with_capacity(64);
    key.push_str(root_tag);
    flatten_into(py, &out, &mut key, root, separator)?;
    Ok(out)
}

/// Convert a single `Node` to the Python object it represents in 1:1 mode.
/// Pure-text leaves collapse to `str`; elements become `dict`; arrays become
/// `list`.
fn node_to_py<'py>(py: Python<'py>, node: &Node) -> PyResult<Bound<'py, PyAny>> {
    match node {
        Node::Text(t) => Ok(PyString::new(py, t).into_any()),
        Node::Element { attrs, children } => {
            if let Some(t) = node.pure_text() {
                return Ok(PyString::new(py, t).into_any());
            }
            let dict = PyDict::new(py);
            for (k, v) in attrs {
                dict.set_item(k.as_ref(), v.as_ref())?;
            }
            for (tag, kids) in children {
                match kids {
                    Children::One(n) => {
                        dict.set_item(tag.as_ref(), node_to_py(py, n)?)?;
                    }
                    Children::Many(v) => {
                        let list = PyList::empty(py);
                        for n in v {
                            list.append(node_to_py(py, n)?)?;
                        }
                        dict.set_item(tag.as_ref(), list)?;
                    }
                }
            }
            Ok(dict.into_any())
        }
    }
}

/// Recursively walk the tree and insert flat `(key, value)` entries into
/// `out`. Reuses `key` via push/truncate semantics.
fn flatten_into<'py>(
    py: Python<'py>,
    out: &Bound<'py, PyDict>,
    key: &mut String,
    node: &Node,
    sep: &str,
) -> PyResult<()> {
    match node {
        Node::Text(t) => {
            out.set_item(&*key, t.as_ref())?;
        }
        Node::Element { attrs, children } => {
            if let Some(t) = node.pure_text() {
                out.set_item(&*key, t)?;
                return Ok(());
            }
            if attrs.is_empty() && children.is_empty() {
                out.set_item(&*key, "")?;
                return Ok(());
            }
            let base_len = key.len();
            for (ak, av) in attrs {
                key.push_str(sep);
                key.push_str(ak);
                out.set_item(&*key, av.as_ref())?;
                key.truncate(base_len);
            }
            for (tag, kids) in children {
                match kids {
                    Children::One(n) => {
                        key.push_str(sep);
                        key.push_str(tag);
                        flatten_into(py, out, key, n, sep)?;
                        key.truncate(base_len);
                    }
                    Children::Many(v) => {
                        for (i, n) in v.iter().enumerate() {
                            use std::fmt::Write;
                            key.push_str(sep);
                            key.push_str(tag);
                            let _ = write!(key, "[{i}]");
                            flatten_into(py, out, key, n, sep)?;
                            key.truncate(base_len);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
