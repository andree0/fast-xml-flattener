//! PyO3 module entry point. Registers the six public functions and marshals
//! parameters / results across the Rust ↔ Python boundary.
//!
//! GIL handling: the pure-Rust pipelines (`to_json`, `to_flatten_json`,
//! `to_csv`, `to_parquet`) are wrapped in `py.allow_threads(...)` so other
//! Python threads can run concurrently. The dict-returning paths hold the
//! GIL because they build `PyDict` objects.

use std::path::PathBuf;

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

/// Parse XML to a 1:1 JSON string that preserves the original structure.
#[pyfunction]
fn to_json(py: Python<'_>, xml: &str) -> PyResult<String> {
    py.detach(|| {
        let (tag, root) = parser::parse(xml)?;
        json::to_json(&tag, &root)
    })
    .map_err(Into::into)
}

/// Parse XML to a flat JSON string using `separator` to join nested tags.
#[pyfunction]
#[pyo3(signature = (xml, separator = "."))]
fn to_flatten_json(py: Python<'_>, xml: &str, separator: &str) -> PyResult<String> {
    py.detach(|| {
        let (tag, root) = parser::parse(xml)?;
        json::to_flatten_json(&tag, &root, separator)
    })
    .map_err(Into::into)
}

/// Parse XML to a 1:1 nested Python `dict` built directly in Rust.
#[pyfunction]
fn to_dict<'py>(py: Python<'py>, xml: &str) -> PyResult<Bound<'py, PyDict>> {
    let (tag, root) = parser::parse(xml).map_err(PyErr::from)?;
    dict::to_dict(py, &tag, &root)
}

/// Parse XML to a flat Python `dict` using `separator`.
#[pyfunction]
#[pyo3(signature = (xml, separator = "."))]
fn to_flatten_dict<'py>(
    py: Python<'py>,
    xml: &str,
    separator: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let (tag, root) = parser::parse(xml).map_err(PyErr::from)?;
    dict::to_flatten_dict(py, &tag, &root, separator)
}

/// Parse XML to a CSV string. Attributes are included iff `include_attrs`.
#[pyfunction]
#[pyo3(signature = (xml, include_attrs = true))]
fn to_csv(py: Python<'_>, xml: &str, include_attrs: bool) -> PyResult<String> {
    py.detach(|| {
        let (tag, root) = parser::parse(xml)?;
        csv_out::to_csv(&tag, &root, include_attrs)
    })
    .map_err(Into::into)
}

/// Parse XML and write the flattened records to a Parquet file at `path`.
#[pyfunction]
#[pyo3(signature = (xml, path, include_attrs = true))]
fn to_parquet(py: Python<'_>, xml: &str, path: PathBuf, include_attrs: bool) -> PyResult<()> {
    py.detach(|| {
        let (tag, root) = parser::parse(xml)?;
        parquet_out::to_parquet(&tag, &root, &path, include_attrs)
    })
    .map_err(Into::into)
}

#[pymodule]
fn fast_xml_flattener(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_json, m)?)?;
    m.add_function(wrap_pyfunction!(to_flatten_json, m)?)?;
    m.add_function(wrap_pyfunction!(to_dict, m)?)?;
    m.add_function(wrap_pyfunction!(to_flatten_dict, m)?)?;
    m.add_function(wrap_pyfunction!(to_csv, m)?)?;
    m.add_function(wrap_pyfunction!(to_parquet, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
