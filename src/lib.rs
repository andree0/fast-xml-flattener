//! PyO3 module entry point. Registers the six public functions and marshals
//! parameters / results across the Rust ↔ Python boundary.
//!
//! GIL handling: the pure-Rust pipelines (`to_json`, `to_flatten_json`,
//! `to_csv`, `to_parquet`) are wrapped in `py.detach(...)` so other
//! Python threads can run concurrently. The dict-returning paths hold the
//! GIL because they build `PyDict` objects.
//!
//! Input: every public function accepts either XML content (str starting with
//! `<`) or a path to an XML file (str not starting with `<`, or a Path-like
//! object). File I/O is done directly in Rust via a buffered reader.

use std::path::PathBuf;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

mod csv_out;
mod dict;
mod error;
mod json;
mod node;
mod parquet_out;
mod parser;
mod record;

use parser::ParserConfig;

/// Resolved input: either an owned XML string or a filesystem path.
enum InputSource {
    Xml(String),
    File(PathBuf),
}

impl InputSource {
    fn parse(self, cfg: &ParserConfig) -> error::Result<(Box<str>, node::Node)> {
        match self {
            InputSource::Xml(s) => parser::parse(&s, cfg),
            InputSource::File(p) => parser::parse_file(&p, cfg),
        }
    }
}

/// Resolve a Python argument (str or path-like) into an `InputSource`.
/// A `str` starting with `<` (after optional whitespace) is treated as XML
/// content; anything else is treated as a file path — same convention as
/// `lxml.etree.parse()`.
fn resolve(xml: &Bound<'_, PyAny>) -> PyResult<InputSource> {
    if let Ok(s) = xml.extract::<String>() {
        let trimmed = s.trim_ascii_start();
        if trimmed.is_empty() || trimmed.starts_with('<') {
            Ok(InputSource::Xml(s))
        } else {
            Ok(InputSource::File(PathBuf::from(s)))
        }
    } else if let Ok(p) = xml.extract::<PathBuf>() {
        Ok(InputSource::File(p))
    } else {
        Err(PyTypeError::new_err(
            "xml must be a str (XML content or file path) or a path-like object",
        ))
    }
}

/// Parse XML to a 1:1 JSON string that preserves the original structure.
#[pyfunction]
#[pyo3(signature = (
    xml,
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
))]
fn to_json(
    py: Python<'_>,
    xml: &Bound<'_, PyAny>,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
) -> PyResult<String> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    py.detach(|| {
        let (tag, root) = source.parse(&cfg)?;
        json::to_json(&tag, &root)
    })
    .map_err(Into::into)
}

/// Parse XML to a flat JSON string using `separator` to join nested tags.
#[pyfunction]
#[pyo3(signature = (
    xml,
    separator = ".",
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
    index_as_key = false,
))]
fn to_flatten_json(
    py: Python<'_>,
    xml: &Bound<'_, PyAny>,
    separator: &str,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
    index_as_key: bool,
) -> PyResult<String> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    py.detach(|| {
        let (tag, root) = source.parse(&cfg)?;
        json::to_flatten_json(&tag, &root, separator, index_as_key)
    })
    .map_err(Into::into)
}

/// Parse XML to a 1:1 nested Python `dict` built directly in Rust.
#[pyfunction]
#[pyo3(signature = (
    xml,
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
))]
fn to_dict<'py>(
    py: Python<'py>,
    xml: &Bound<'_, PyAny>,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    let (tag, root) = source.parse(&cfg).map_err(PyErr::from)?;
    dict::to_dict(py, &tag, &root)
}

/// Parse XML to a flat Python `dict` using `separator`.
#[pyfunction]
#[pyo3(signature = (
    xml,
    separator = ".",
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
    index_as_key = false,
))]
fn to_flatten_dict<'py>(
    py: Python<'py>,
    xml: &Bound<'_, PyAny>,
    separator: &str,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
    index_as_key: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    let (tag, root) = source.parse(&cfg).map_err(PyErr::from)?;
    dict::to_flatten_dict(py, &tag, &root, separator, index_as_key)
}

/// Parse XML to a CSV string. Attributes are included iff `include_attrs`.
#[pyfunction]
#[pyo3(signature = (
    xml,
    include_attrs = true,
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
    index_as_key = false,
))]
fn to_csv(
    py: Python<'_>,
    xml: &Bound<'_, PyAny>,
    include_attrs: bool,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
    index_as_key: bool,
) -> PyResult<String> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    py.detach(|| {
        let (tag, root) = source.parse(&cfg)?;
        csv_out::to_csv(&tag, &root, include_attrs, index_as_key)
    })
    .map_err(Into::into)
}

/// Parse XML and write the flattened records to a Parquet file at `path`.
#[pyfunction]
#[pyo3(signature = (
    xml,
    path,
    include_attrs = true,
    *,
    strip_whitespace = true,
    keep_namespace_declarations = false,
    index_as_key = false,
))]
fn to_parquet(
    py: Python<'_>,
    xml: &Bound<'_, PyAny>,
    path: PathBuf,
    include_attrs: bool,
    strip_whitespace: bool,
    keep_namespace_declarations: bool,
    index_as_key: bool,
) -> PyResult<()> {
    let source = resolve(xml)?;
    let cfg = ParserConfig {
        strip_whitespace,
        keep_namespace_declarations,
    };
    py.detach(|| {
        let (tag, root) = source.parse(&cfg)?;
        parquet_out::to_parquet(&tag, &root, &path, include_attrs, index_as_key)
    })
    .map_err(Into::into)
}

/// Return the local name of the document root, with any namespace prefix
/// stripped. Reads only as far as the first start tag.
#[pyfunction]
fn get_root_tag_name(py: Python<'_>, xml: &Bound<'_, PyAny>) -> PyResult<String> {
    let source = resolve(xml)?;
    py.detach(|| match source {
        InputSource::Xml(s) => parser::root_tag_name_from_str(&s),
        InputSource::File(p) => parser::root_tag_name_from_file(&p),
    })
    .map_err(Into::into)
}

// LCOV_EXCL_START
#[pymodule]
fn _fast_xml_flattener(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_json, m)?)?;
    m.add_function(wrap_pyfunction!(to_flatten_json, m)?)?;
    m.add_function(wrap_pyfunction!(to_dict, m)?)?;
    m.add_function(wrap_pyfunction!(to_flatten_dict, m)?)?;
    m.add_function(wrap_pyfunction!(to_csv, m)?)?;
    m.add_function(wrap_pyfunction!(to_parquet, m)?)?;
    m.add_function(wrap_pyfunction!(get_root_tag_name, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
// LCOV_EXCL_STOP
